mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use spoolbook_rs::export_import::{insert_json_row, row_to_json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

fn empty_manifest() -> serde_json::Value {
    let mut m = serde_json::json!({ "version": env!("CARGO_PKG_VERSION") });
    for table in spoolbook_rs::export_import::EXPORT_TABLES {
        m[table] = serde_json::json!([]);
    }
    m
}

fn build_zip(manifest: &serde_json::Value) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        std::io::Write::write_all(&mut zip, manifest.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf
}

async fn post_zip(pool: &sqlx::SqlitePool, uri: &str, zip_bytes: &[u8]) -> (StatusCode, serde_json::Value) {
    let boundary = "spoolbook-rs-test-boundary";
    let mut body = Vec::new();
    std::io::Write::write_all(
        &mut body,
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"export.zip\"\r\nContent-Type: application/zip\r\n\r\n").as_bytes(),
    )
    .unwrap();
    body.extend_from_slice(zip_bytes);
    std::io::Write::write_all(&mut body, format!("\r\n--{boundary}--\r\n").as_bytes()).unwrap();

    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", format!("multipart/form-data; boundary={boundary}"))
                .header("cookie", common::auth_cookie_header(pool).await)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

#[tokio::test]
async fn import_preview_reports_matched_vs_new_for_dedupeable_tables() {
    let pool = test_pool().await;
    sqlx::query("INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO printers (name) VALUES ('Pitus')").execute(&pool).await.unwrap();

    let mut manifest = empty_manifest();
    manifest["filaments"] = serde_json::json!([
        { "id": 1, "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" },
        { "id": 2, "brand": "Slic3D", "material": "PETG", "variant": null, "color": "Clear" }
    ]);
    manifest["printers"] = serde_json::json!([
        { "id": 1, "name": "Pitus", "model": null, "ip_address": null, "access_code": null, "serial_number": null },
        { "id": 2, "name": "New Printer", "model": null, "ip_address": null, "access_code": null, "serial_number": null }
    ]);

    let (status, body) = post_zip(&pool, "/api/import/preview", &build_zip(&manifest)).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["filaments"]["total"], 2);
    assert_eq!(body["filaments"]["new"], 1, "one matches the existing Bambu Lab PLA Basic Black by natural key");
    assert_eq!(body["printers"]["total"], 2);
    assert_eq!(body["printers"]["new"], 1, "one matches the existing 'Pitus' by name");
}

#[tokio::test]
async fn import_preview_reports_every_row_as_new_for_non_dedupeable_tables() {
    let pool = test_pool().await;
    let mut manifest = empty_manifest();
    manifest["spools"] = serde_json::json!([{ "id": 1, "filament_id": 1 }, { "id": 2, "filament_id": 1 }]);

    let (status, body) = post_zip(&pool, "/api/import/preview", &build_zip(&manifest)).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["spools"]["total"], 2);
    assert_eq!(body["spools"]["new"], 2);
}

// Same lookup-table shape as Filament/Printer -- matches the app's own filament-creation flow
// (ensure_color_exists), which already dedupes by name before inserting. Found via a real test:
// self-importing a real DB (a natural sanity check, not a supported scenario) added 1391
// duplicate colors before this dedupe rule existed.
#[tokio::test]
async fn import_preview_dedupes_filament_colors_by_name() {
    let pool = test_pool().await;
    sqlx::query("INSERT INTO filament_colors (name, hex) VALUES ('Black', '#000000')").execute(&pool).await.unwrap();
    let mut manifest = empty_manifest();
    manifest["filament_colors"] = serde_json::json!([{ "id": 1, "name": "Black", "hex": "#000000" }, { "id": 2, "name": "Galaxy Purple", "hex": "#5B3A8C" }]);

    let (status, body) = post_zip(&pool, "/api/import/preview", &build_zip(&manifest)).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["filament_colors"]["total"], 2);
    assert_eq!(body["filament_colors"]["new"], 1);
}

#[tokio::test]
async fn import_preview_rejects_a_manifest_from_a_newer_version() {
    let pool = test_pool().await;
    let mut manifest = empty_manifest();
    manifest["version"] = serde_json::json!("999.0.0");

    let (status, body) = post_zip(&pool, "/api/import/preview", &build_zip(&manifest)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
}

#[tokio::test]
async fn import_preview_requires_auth() {
    let pool = test_pool().await;
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/api/import/preview").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn build_zip_with_file(manifest: &serde_json::Value, file_name: &str, file_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        std::io::Write::write_all(&mut zip, manifest.to_string().as_bytes()).unwrap();
        zip.start_file(format!("files/{file_name}"), options).unwrap();
        std::io::Write::write_all(&mut zip, file_bytes).unwrap();
        zip.finish().unwrap();
    }
    buf
}

#[tokio::test]
async fn import_commit_merges_the_full_chain_with_dedupe_and_fk_remap() {
    let pool = test_pool().await;
    let existing_filament_id: i64 = sqlx::query_scalar("INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black') RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    let existing_printer_id: i64 = sqlx::query_scalar("INSERT INTO printers (name) VALUES ('Pitus') RETURNING id").fetch_one(&pool).await.unwrap();

    let mut manifest = empty_manifest();
    // Source id 1 matches the target's existing filament by natural key; source id 2 is new.
    manifest["filaments"] = serde_json::json!([
        { "id": 1, "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" },
        { "id": 2, "brand": "Slic3D", "material": "PETG", "variant": null, "color": "Clear" }
    ]);
    manifest["printers"] = serde_json::json!([{ "id": 1, "name": "Pitus", "model": null, "ip_address": null, "access_code": null, "serial_number": null }]);
    // Spool references the NEW filament (source id 2); Profile references the MATCHED one (source id 1).
    manifest["spools"] = serde_json::json!([{ "id": 1, "filament_id": 2, "lot_code": null, "purchased_at": null, "opened_at": null, "emptied_at": null, "weight_grams": null, "diameter_mm": null, "notes": null, "created_at": "2026-01-01T00:00:00Z" }]);
    manifest["print_profiles"] = serde_json::json!([{ "id": 1, "filament_id": 1, "name": "Standard", "nozzle_temp_c": 220 }]);
    manifest["projects"] = serde_json::json!([{ "id": 1, "file_path": "/some/source-only/path/deadbeef.3mf", "file_name": "part.3mf", "last_known_write_time_utc": "2026-01-01T00:00:00Z", "last_known_file_size_bytes": 4 }]);
    manifest["prints"] = serde_json::json!([{ "id": 1, "profile_id": 1, "spool_id": 1, "printer_id": 1, "project_id": 1, "started_at": "2026-01-01T00:00:00Z", "status": "Success", "created_at": "2026-01-01T00:00:00Z" }]);
    manifest["print_failure_modes"] = serde_json::json!([{ "id": 1, "print_id": 1, "mode": "Stringing" }]);
    manifest["print_hourly_weather"] = serde_json::json!([{ "id": 1, "print_id": 1, "hour": "2026-01-01T00:00", "temp_c": 20.0, "humidity_pct": 50.0 }]);

    let zip_bytes = build_zip_with_file(&manifest, "deadbeef.3mf", b"fake");
    let (status, body) = post_zip(&pool, "/api/import/commit", &zip_bytes).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    // Filament dedupe: still exactly 2 rows (the matched one wasn't duplicated).
    let filament_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM filaments").fetch_one(&pool).await.unwrap();
    assert_eq!(filament_count, 2);
    let new_filament_id: i64 = sqlx::query_scalar("SELECT id FROM filaments WHERE brand = 'Slic3D'").fetch_one(&pool).await.unwrap();

    // Printer dedupe: still exactly 1 row.
    let printer_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM printers").fetch_one(&pool).await.unwrap();
    assert_eq!(printer_count, 1);

    // Spool's FK was remapped from source filament id 2 -> the real new target filament id.
    let spool_filament_id: i64 = sqlx::query_scalar("SELECT filament_id FROM spools").fetch_one(&pool).await.unwrap();
    assert_eq!(spool_filament_id, new_filament_id);

    // Profile's FK was remapped from source filament id 1 -> the existing matched filament's real id.
    let profile_filament_id: i64 = sqlx::query_scalar("SELECT filament_id FROM print_profiles").fetch_one(&pool).await.unwrap();
    assert_eq!(profile_filament_id, existing_filament_id);

    // Project's file was written into the target's real storage dir, and its file_path column
    // points there -- not at the source-only path from the manifest.
    let project_path: String = sqlx::query_scalar("SELECT file_path FROM projects").fetch_one(&pool).await.unwrap();
    assert!(project_path.ends_with("deadbeef.3mf"), "{project_path}");
    assert_ne!(project_path, "/some/source-only/path/deadbeef.3mf");
    assert_eq!(std::fs::read(&project_path).unwrap(), b"fake");

    // Print's every FK remapped correctly.
    let print_row = sqlx::query("SELECT profile_id, spool_id, printer_id, project_id FROM prints").fetch_one(&pool).await.unwrap();
    let profile_id: i64 = print_row.get("profile_id");
    let spool_id: i64 = print_row.get("spool_id");
    let printer_id: i64 = print_row.get("printer_id");
    let project_id: i64 = print_row.get("project_id");
    assert_eq!(printer_id, existing_printer_id);
    let real_profile_id: i64 = sqlx::query_scalar("SELECT id FROM print_profiles").fetch_one(&pool).await.unwrap();
    let real_spool_id: i64 = sqlx::query_scalar("SELECT id FROM spools").fetch_one(&pool).await.unwrap();
    let real_project_id: i64 = sqlx::query_scalar("SELECT id FROM projects").fetch_one(&pool).await.unwrap();
    assert_eq!(profile_id, real_profile_id);
    assert_eq!(spool_id, real_spool_id);
    assert_eq!(project_id, real_project_id);

    // Failure mode and hourly weather both remapped to the real print id.
    let real_print_id: i64 = sqlx::query_scalar("SELECT id FROM prints").fetch_one(&pool).await.unwrap();
    let fm_print_id: i64 = sqlx::query_scalar("SELECT print_id FROM print_failure_modes").fetch_one(&pool).await.unwrap();
    let hw_print_id: i64 = sqlx::query_scalar("SELECT print_id FROM print_hourly_weather").fetch_one(&pool).await.unwrap();
    assert_eq!(fm_print_id, real_print_id);
    assert_eq!(hw_print_id, real_print_id);
}

#[tokio::test]
async fn import_commit_requires_auth() {
    let pool = test_pool().await;
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(Request::builder().method("POST").uri("/api/import/commit").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

async fn get_bytes(pool: &sqlx::SqlitePool, uri: &str, authed: bool) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if authed {
        builder = builder.header("cookie", common::auth_cookie_header(pool).await);
    }
    let response = spoolbook_rs::app(pool.clone()).oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn export_requires_auth() {
    let pool = test_pool().await;
    let (status, _) = get_bytes(&pool, "/api/export", false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// One row per exported table, wired into a real dependency chain, proving the generic
// row_to_json/table dump mechanism works for every table -- including print_profiles' ~140
// columns -- without any table-specific handling.
async fn seed_one_row_per_table(pool: &sqlx::SqlitePool) {
    let filament_id: i64 = sqlx::query_scalar("INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO filament_colors (name, hex) VALUES ('Black', '#000000')").execute(pool).await.unwrap();
    let printer_id: i64 = sqlx::query_scalar("INSERT INTO printers (name) VALUES ('Pitus') RETURNING id").fetch_one(pool).await.unwrap();
    let spool_id: i64 = sqlx::query_scalar("INSERT INTO spools (filament_id) VALUES (?1) RETURNING id").bind(filament_id).fetch_one(pool).await.unwrap();
    let profile_id: i64 = sqlx::query_scalar("INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, 'Standard', 220) RETURNING id")
        .bind(filament_id)
        .fetch_one(pool)
        .await
        .unwrap();
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes) VALUES ('/data/projects/abc.3mf', 'abc.3mf', '2026-01-01T00:00:00Z', 100) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let print_id: i64 = sqlx::query_scalar(
        "INSERT INTO prints (profile_id, spool_id, printer_id, project_id, started_at, status) VALUES (?1, ?2, ?3, ?4, '2026-01-01T00:00:00Z', 'Success') RETURNING id",
    )
    .bind(profile_id)
    .bind(spool_id)
    .bind(printer_id)
    .bind(project_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO print_failure_modes (print_id, mode) VALUES (?1, 'Stringing')").bind(print_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO print_hourly_weather (print_id, hour, temp_c, humidity_pct) VALUES (?1, '2026-01-01T00:00', 20.0, 50.0)").bind(print_id).execute(pool).await.unwrap();
}

#[tokio::test]
async fn export_includes_every_mergeable_table_including_print_profiles() {
    let pool = test_pool().await;
    seed_one_row_per_table(&pool).await;

    let (status, bytes) = get_bytes(&pool, "/api/export", true).await;

    assert_eq!(status, StatusCode::OK);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("valid zip");
    let mut manifest_file = archive.by_name("manifest.json").expect("manifest.json present");
    let mut manifest_str = String::new();
    std::io::Read::read_to_string(&mut manifest_file, &mut manifest_str).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();

    for table in spoolbook_rs::export_import::EXPORT_TABLES {
        let rows = manifest[table].as_array().unwrap_or_else(|| panic!("{table} missing from manifest"));
        assert_eq!(rows.len(), 1, "{table} should have exactly the one seeded row");
    }
    assert_eq!(manifest["print_profiles"][0]["nozzle_temp_c"], 220);
    assert_eq!(manifest["print_profiles"][0]["name"], "Standard");
    assert_eq!(manifest["prints"][0]["status"], "Success");
}

#[tokio::test]
async fn export_bundles_the_projects_3mf_bytes_under_files() {
    let pool = test_pool().await;
    let dir = std::env::temp_dir().join(format!("spoolbook-rs-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let stored_path = dir.join("deadbeef.3mf");
    std::fs::write(&stored_path, b"fake 3mf bytes").unwrap();
    sqlx::query(
        "INSERT INTO projects (file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes) VALUES (?1, 'my-part.3mf', '2026-01-01T00:00:00Z', 14)",
    )
    .bind(stored_path.to_str().unwrap())
    .execute(&pool)
    .await
    .unwrap();

    let (status, bytes) = get_bytes(&pool, "/api/export", true).await;

    assert_eq!(status, StatusCode::OK);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("valid zip");
    let mut file = archive.by_name("files/deadbeef.3mf").expect("bundled 3mf file present");
    let mut content = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut content).unwrap();
    assert_eq!(content, b"fake 3mf bytes");
}

#[tokio::test]
async fn export_produces_a_zip_with_a_manifest_containing_seeded_filaments() {
    let pool = test_pool().await;
    sqlx::query("INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black')").execute(&pool).await.unwrap();

    let (status, bytes) = get_bytes(&pool, "/api/export", true).await;

    assert_eq!(status, StatusCode::OK);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("valid zip");
    let mut manifest_file = archive.by_name("manifest.json").expect("manifest.json present");
    let mut manifest_str = String::new();
    std::io::Read::read_to_string(&mut manifest_file, &mut manifest_str).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();

    assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    let filaments = manifest["filaments"].as_array().expect("filaments array");
    assert_eq!(filaments.len(), 1);
    assert_eq!(filaments[0]["brand"], "Bambu Lab");
}

#[tokio::test]
async fn row_to_json_and_insert_json_row_round_trip_a_filament() {
    let pool = test_pool().await;
    sqlx::query("INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black')")
        .execute(&pool)
        .await
        .unwrap();
    let row = sqlx::query("SELECT * FROM filaments WHERE brand = 'Bambu Lab'").fetch_one(&pool).await.unwrap();
    let json = row_to_json(&row);

    assert_eq!(json["brand"], "Bambu Lab");
    assert_eq!(json["material"], "PLA");
    assert_eq!(json["variant"], "Basic");
    assert_eq!(json["color"], "Black");
    assert!(json["id"].is_i64(), "row_to_json includes id -- insert_json_row's job to skip it");

    let new_id = insert_json_row(&pool, "filaments", &json).await;

    let original_id = row.get::<i64, _>("id");
    assert_ne!(new_id, original_id, "insert_json_row assigns a fresh id, doesn't reuse the source's");
    let reinserted = sqlx::query("SELECT brand, material, variant, color FROM filaments WHERE id = ?1").bind(new_id).fetch_one(&pool).await.unwrap();
    assert_eq!(reinserted.get::<String, _>("brand"), "Bambu Lab");
    assert_eq!(reinserted.get::<String, _>("material"), "PLA");
    assert_eq!(reinserted.get::<Option<String>, _>("variant").as_deref(), Some("Basic"));
    assert_eq!(reinserted.get::<String, _>("color"), "Black");
}

#[tokio::test]
async fn row_to_json_and_insert_json_row_handle_null_columns() {
    let pool = test_pool().await;
    sqlx::query("INSERT INTO filaments (brand, material, variant, color) VALUES ('Generic', 'PETG', NULL, 'Clear')").execute(&pool).await.unwrap();
    let row = sqlx::query("SELECT * FROM filaments WHERE brand = 'Generic'").fetch_one(&pool).await.unwrap();
    let json = row_to_json(&row);
    assert!(json["variant"].is_null());

    let new_id = insert_json_row(&pool, "filaments", &json).await;
    let reinserted = sqlx::query("SELECT variant FROM filaments WHERE id = ?1").bind(new_id).fetch_one(&pool).await.unwrap();
    assert_eq!(reinserted.get::<Option<String>, _>("variant"), None);
}

#[tokio::test]
async fn row_to_json_and_insert_json_row_round_trip_real_columns_and_fk_columns() {
    let pool = test_pool().await;
    let filament_id: i64 = sqlx::query_scalar("INSERT INTO filaments (brand, material, color) VALUES ('B', 'PLA', 'Red') RETURNING id").fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO spools (filament_id, diameter_mm) VALUES (?1, 1.75)").bind(filament_id).execute(&pool).await.unwrap();
    let row = sqlx::query("SELECT * FROM spools").fetch_one(&pool).await.unwrap();
    let json = row_to_json(&row);

    assert_eq!(json["diameter_mm"].as_f64(), Some(1.75));
    assert_eq!(json["filament_id"].as_i64(), Some(filament_id));

    let new_id = insert_json_row(&pool, "spools", &json).await;
    let reinserted = sqlx::query("SELECT filament_id, diameter_mm FROM spools WHERE id = ?1").bind(new_id).fetch_one(&pool).await.unwrap();
    assert_eq!(reinserted.get::<f64, _>("diameter_mm"), 1.75);
    assert_eq!(reinserted.get::<i64, _>("filament_id"), filament_id, "no remap applied -- insert_json_row just inserts what it's given");
}
