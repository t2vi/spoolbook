// Data export/import (docs/adr/0033): a zip bundle merged into the target install rather than
// replacing it. print_profiles alone has ~140 columns -- rather than hand-writing/maintaining a
// typed struct twice over (once to serialize, once to bind an INSERT) for every mergeable table,
// these two functions read/write any row generically by column name, reused across every table.
use axum::extract::{Multipart, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde_json::{Map, Value};
use sqlx::sqlite::SqliteRow;
use sqlx::{Column, Row, SqlitePool, TypeInfo, ValueRef};
use std::io::Write;

// "1.3.0" -> (1, 3, 0). Malformed/missing pieces read as 0 -- good enough for the one comparison
// this exists for (reject a manifest newer than this build), not general semver handling.
fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut parts = v.split('.').map(|p| p.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

async fn read_manifest_from_zip(bytes: &[u8]) -> Result<Value, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Not a valid export file: {e}"))?;
    let mut manifest_file = archive.by_name("manifest.json").map_err(|_| "Export file is missing manifest.json".to_string())?;
    let mut manifest_str = String::new();
    std::io::Read::read_to_string(&mut manifest_file, &mut manifest_str).map_err(|e| e.to_string())?;
    serde_json::from_str(&manifest_str).map_err(|e| format!("manifest.json is malformed: {e}"))
}

async fn read_uploaded_zip(multipart: &mut Multipart) -> Result<Vec<u8>, String> {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            return field.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string());
        }
    }
    Err("No file provided.".to_string())
}

// Some(target id) when a row with the same natural key already exists in the target; None means
// this row is new. Shared by preview (just needs new-vs-matched counts) and commit (also needs
// the actual matched id, to remap children's foreign keys onto it instead of inserting a dupe).
async fn find_matching_filament(pool: &SqlitePool, row: &Map<String, Value>) -> Option<i64> {
    sqlx::query_scalar("SELECT id FROM filaments WHERE brand = ?1 AND material = ?2 AND variant IS ?3 AND color = ?4")
        .bind(row["brand"].as_str())
        .bind(row["material"].as_str())
        .bind(row["variant"].as_str())
        .bind(row["color"].as_str())
        .fetch_optional(pool)
        .await
        .expect("query failed")
}

async fn find_matching_printer(pool: &SqlitePool, row: &Map<String, Value>) -> Option<i64> {
    sqlx::query_scalar("SELECT id FROM printers WHERE name = ?1").bind(row["name"].as_str()).fetch_optional(pool).await.expect("query failed")
}

// Same lookup-table shape as Filament/Printer, and dedupes the same way the app's own
// filament-creation flow already does (filaments.rs's ensure_color_exists).
async fn find_matching_filament_color(pool: &SqlitePool, row: &Map<String, Value>) -> Option<i64> {
    sqlx::query_scalar("SELECT id FROM filament_colors WHERE name = ?1").bind(row["name"].as_str()).fetch_optional(pool).await.expect("query failed")
}

async fn table_new_count(pool: &SqlitePool, table: &str, rows: &[Value]) -> usize {
    match table {
        "filaments" => {
            let mut n = 0;
            for row in rows {
                if find_matching_filament(pool, row.as_object().unwrap()).await.is_none() {
                    n += 1;
                }
            }
            n
        }
        "printers" => {
            let mut n = 0;
            for row in rows {
                if find_matching_printer(pool, row.as_object().unwrap()).await.is_none() {
                    n += 1;
                }
            }
            n
        }
        "filament_colors" => {
            let mut n = 0;
            for row in rows {
                if find_matching_filament_color(pool, row.as_object().unwrap()).await.is_none() {
                    n += 1;
                }
            }
            n
        }
        _ => rows.len(),
    }
}

// Rewrites `obj[col]` (a source-install row id) to whatever the target install actually assigned
// that same logical row, per `map` (built while processing the parent table earlier in
// EXPORT_TABLES' dependency order). A null/missing source value stays null -- covers nullable FKs
// like Print.project_id.
fn remap_fk(obj: &mut Map<String, Value>, col: &str, map: &std::collections::HashMap<i64, i64>) {
    if let Some(source_id) = obj.get(col).and_then(Value::as_i64) {
        obj.insert(col.to_string(), map.get(&source_id).map(|id| Value::from(*id)).unwrap_or(Value::Null));
    }
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/export", get(export)).route("/api/import/preview", post(preview)).route("/api/import/commit", post(commit))
}

async fn preview(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, mut multipart: Multipart) -> (StatusCode, Json<Value>) {
    let bytes = match read_uploaded_zip(&mut multipart).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": e }))),
    };
    let manifest = match read_manifest_from_zip(&bytes).await {
        Ok(m) => m,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": e }))),
    };

    let source_version = manifest["version"].as_str().unwrap_or("0.0.0");
    if parse_version(source_version) > parse_version(env!("CARGO_PKG_VERSION")) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": format!("Export was made on spoolbook {source_version}, newer than this install ({}). Update first.", env!("CARGO_PKG_VERSION")) })),
        );
    }

    let mut result = Map::new();
    for table in EXPORT_TABLES {
        let rows = manifest[table].as_array().cloned().unwrap_or_default();
        let new_count = table_new_count(&pool, table, &rows).await;
        result.insert((*table).to_string(), serde_json::json!({ "total": rows.len(), "new": new_count }));
    }
    (StatusCode::OK, Json(Value::Object(result)))
}

fn rows_of<'a>(manifest: &'a Value, table: &str) -> Vec<Map<String, Value>> {
    manifest[table].as_array().cloned().unwrap_or_default().into_iter().map(|v| v.as_object().unwrap().clone()).collect()
}

async fn commit(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, mut multipart: Multipart) -> (StatusCode, Json<Value>) {
    let bytes = match read_uploaded_zip(&mut multipart).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": e }))),
    };
    let manifest = match read_manifest_from_zip(&bytes).await {
        Ok(m) => m,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": e }))),
    };
    let source_version = manifest["version"].as_str().unwrap_or("0.0.0");
    if parse_version(source_version) > parse_version(env!("CARGO_PKG_VERSION")) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": format!("Export was made on spoolbook {source_version}, newer than this install ({}). Update first.", env!("CARGO_PKG_VERSION")) })),
        );
    }

    let mut summary = Map::new();
    let mut filament_map = std::collections::HashMap::new();
    let mut printer_map = std::collections::HashMap::new();
    let mut spool_map = std::collections::HashMap::new();
    let mut profile_map = std::collections::HashMap::new();
    let mut project_map = std::collections::HashMap::new();
    let mut print_map = std::collections::HashMap::new();

    let mut inserted = 0;
    let mut matched = 0;
    for mut row in rows_of(&manifest, "filaments") {
        let source_id = row["id"].as_i64().unwrap();
        let target_id = match find_matching_filament(&pool, &row).await {
            Some(id) => {
                matched += 1;
                id
            }
            None => {
                inserted += 1;
                row.remove("id");
                insert_json_row(&pool, "filaments", &row).await
            }
        };
        filament_map.insert(source_id, target_id);
    }
    summary.insert("filaments".to_string(), serde_json::json!({ "inserted": inserted, "matched": matched }));

    let mut inserted = 0;
    let mut matched = 0;
    for mut row in rows_of(&manifest, "filament_colors") {
        if find_matching_filament_color(&pool, &row).await.is_some() {
            matched += 1;
            continue;
        }
        row.remove("id");
        insert_json_row(&pool, "filament_colors", &row).await;
        inserted += 1;
    }
    summary.insert("filament_colors".to_string(), serde_json::json!({ "inserted": inserted, "matched": matched }));

    let mut inserted = 0;
    let mut matched = 0;
    for mut row in rows_of(&manifest, "printers") {
        let source_id = row["id"].as_i64().unwrap();
        let target_id = match find_matching_printer(&pool, &row).await {
            Some(id) => {
                matched += 1;
                id
            }
            None => {
                inserted += 1;
                row.remove("id");
                insert_json_row(&pool, "printers", &row).await
            }
        };
        printer_map.insert(source_id, target_id);
    }
    summary.insert("printers".to_string(), serde_json::json!({ "inserted": inserted, "matched": matched }));

    let mut inserted = 0;
    for mut row in rows_of(&manifest, "spools") {
        let source_id = row["id"].as_i64().unwrap();
        remap_fk(&mut row, "filament_id", &filament_map);
        row.remove("id");
        let target_id = insert_json_row(&pool, "spools", &row).await;
        spool_map.insert(source_id, target_id);
        inserted += 1;
    }
    summary.insert("spools".to_string(), serde_json::json!({ "inserted": inserted, "matched": 0 }));

    let mut inserted = 0;
    for mut row in rows_of(&manifest, "print_profiles") {
        let source_id = row["id"].as_i64().unwrap();
        remap_fk(&mut row, "filament_id", &filament_map);
        row.remove("id");
        let target_id = insert_json_row(&pool, "print_profiles", &row).await;
        profile_map.insert(source_id, target_id);
        inserted += 1;
    }
    summary.insert("print_profiles".to_string(), serde_json::json!({ "inserted": inserted, "matched": 0 }));

    let mut inserted = 0;
    let mut files_archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).ok();
    let storage_dir = crate::project_upload::storage_dir();
    for mut row in rows_of(&manifest, "projects") {
        let source_id = row["id"].as_i64().unwrap();
        let source_path = row["file_path"].as_str().unwrap_or("").to_string();
        if let Some(name) = std::path::Path::new(&source_path).file_name().and_then(|n| n.to_str()) {
            let target_path = storage_dir.join(name);
            if !target_path.exists() {
                if let Some(archive) = files_archive.as_mut() {
                    if let Ok(mut zip_file) = archive.by_name(&format!("files/{name}")) {
                        let mut content = Vec::new();
                        if std::io::Read::read_to_end(&mut zip_file, &mut content).is_ok() {
                            std::fs::write(&target_path, &content).ok();
                        }
                    }
                }
            }
            row.insert("file_path".to_string(), Value::from(target_path.to_str().unwrap_or_default()));
        }
        row.remove("id");
        let target_id = insert_json_row(&pool, "projects", &row).await;
        project_map.insert(source_id, target_id);
        inserted += 1;
    }
    summary.insert("projects".to_string(), serde_json::json!({ "inserted": inserted, "matched": 0 }));

    let mut inserted = 0;
    for mut row in rows_of(&manifest, "prints") {
        let source_id = row["id"].as_i64().unwrap();
        remap_fk(&mut row, "profile_id", &profile_map);
        remap_fk(&mut row, "spool_id", &spool_map);
        remap_fk(&mut row, "printer_id", &printer_map);
        remap_fk(&mut row, "project_id", &project_map);
        row.remove("id");
        let target_id = insert_json_row(&pool, "prints", &row).await;
        print_map.insert(source_id, target_id);
        inserted += 1;
    }
    summary.insert("prints".to_string(), serde_json::json!({ "inserted": inserted, "matched": 0 }));

    let mut inserted = 0;
    for mut row in rows_of(&manifest, "print_failure_modes") {
        remap_fk(&mut row, "print_id", &print_map);
        row.remove("id");
        insert_json_row(&pool, "print_failure_modes", &row).await;
        inserted += 1;
    }
    summary.insert("print_failure_modes".to_string(), serde_json::json!({ "inserted": inserted, "matched": 0 }));

    let mut inserted = 0;
    for mut row in rows_of(&manifest, "print_hourly_weather") {
        remap_fk(&mut row, "print_id", &print_map);
        row.remove("id");
        insert_json_row(&pool, "print_hourly_weather", &row).await;
        inserted += 1;
    }
    summary.insert("print_hourly_weather".to_string(), serde_json::json!({ "inserted": inserted, "matched": 0 }));

    (StatusCode::OK, Json(serde_json::json!({ "ok": true, "tables": summary })))
}

// Dependency order: a child row's foreign key must point at an already-inserted parent, so
// import processes tables in this order. Export doesn't care about order but reuses the same
// list so both sides agree on exactly what's in scope. Deliberately excludes users/sessions (the
// target has its own login), app_settings (single-row host config, not a merge candidate), and
// printer_jobs/printer_readings (transient MQTT scaffolding per ADR-0032 -- a Print's permanent
// telemetry record is already covered by exporting `prints`, which carries telemetry_json).
pub const EXPORT_TABLES: &[&str] = &[
    "filaments",
    "filament_colors",
    "printers",
    "spools",
    "print_profiles",
    "projects",
    "prints",
    "print_failure_modes",
    "print_hourly_weather",
];

pub fn row_to_json(row: &SqliteRow) -> Map<String, Value> {
    let mut map = Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let raw = row.try_get_raw(i).expect("column index out of range");
        let value = if raw.is_null() {
            Value::Null
        } else {
            match raw.type_info().name() {
                "INTEGER" | "BOOLEAN" => row.try_get::<i64, _>(i).map(Value::from).unwrap_or(Value::Null),
                "REAL" => row.try_get::<f64, _>(i).map(Value::from).unwrap_or(Value::Null),
                "BLOB" => row.try_get::<Vec<u8>, _>(i).map(|b| Value::from(base64::engine::general_purpose::STANDARD.encode(&b))).unwrap_or(Value::Null),
                _ => row.try_get::<String, _>(i).map(Value::from).unwrap_or(Value::Null),
            }
        };
        map.insert(col.name().to_string(), value);
    }
    map
}

// Inserts every field in `row` except "id" (a fresh one is always assigned) and returns it.
// Callers own foreign-key remapping -- this only ever inserts whatever's already in the map.
pub async fn insert_json_row(pool: &SqlitePool, table: &str, row: &Map<String, Value>) -> i64 {
    let columns: Vec<&String> = row.keys().filter(|k| k.as_str() != "id").collect();
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
    let sql = format!("INSERT INTO {table} ({}) VALUES ({}) RETURNING id", columns.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "), placeholders.join(", "));

    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    for col in &columns {
        query = match &row[*col] {
            Value::Null => query.bind(None::<String>),
            Value::Number(n) if n.is_i64() => query.bind(n.as_i64()),
            Value::Number(n) => query.bind(n.as_f64()),
            Value::String(s) => query.bind(s.clone()),
            other => panic!("insert_json_row: unsupported value for column {col}: {other:?}"),
        };
    }
    query.fetch_one(pool).await.expect("insert failed")
}

async fn table_rows(pool: &SqlitePool, table: &str) -> Vec<Map<String, Value>> {
    sqlx::query(&format!("SELECT * FROM {table}")).fetch_all(pool).await.expect("query failed").iter().map(row_to_json).collect()
}

async fn export(_editor: crate::auth::Editor, State(pool): State<SqlitePool>) -> Response {
    let mut manifest = Map::new();
    manifest.insert("version".to_string(), Value::from(env!("CARGO_PKG_VERSION")));
    for table in EXPORT_TABLES {
        let rows = table_rows(&pool, table).await.into_iter().map(Value::Object).collect();
        manifest.insert((*table).to_string(), Value::Array(rows));
    }

    let project_paths: Vec<String> = manifest["projects"].as_array().unwrap().iter().filter_map(|p| p["file_path"].as_str().map(str::to_string)).collect();

    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options).expect("zip write failed");
        zip.write_all(serde_json::to_string(&manifest).expect("serialize failed").as_bytes()).expect("zip write failed");

        // Skips gracefully rather than failing the whole export when a Project's file is
        // missing/unreadable -- same drift-tolerant posture as the rest of the Project model.
        for path in &project_paths {
            let Some(name) = std::path::Path::new(path).file_name().and_then(|n| n.to_str()) else { continue };
            let Ok(bytes) = std::fs::read(path) else { continue };
            zip.start_file(format!("files/{name}"), options).expect("zip write failed");
            zip.write_all(&bytes).expect("zip write failed");
        }

        zip.finish().expect("zip finish failed");
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/zip"), (header::CONTENT_DISPOSITION, "attachment; filename=\"spoolbook-export.zip\"")],
        buf,
    )
        .into_response()
}
