mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use std::io::Write;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to open in-memory db");
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn send(pool: &sqlx::SqlitePool, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", common::auth_cookie_header(pool).await)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

// Builds a minimal real .3mf-shaped zip (not a mock) with one plate + thumbnail, matching what
// ProjectService.ReadPlates actually parses in .NET. Returns the file path.
fn write_fixture_3mf(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("3D/3dmodel.model", options).unwrap();
    zip.write_all(b"<mesh>fake geometry</mesh>").unwrap();

    zip.start_file("Metadata/model_settings.config", options).unwrap();
    zip.write_all(
        br#"<config>
            <plate>
                <metadata key="plater_id" value="1"/>
                <metadata key="plater_name" value="Plate 1"/>
                <metadata key="thumbnail_file" value="Metadata/plate_1.png"/>
            </plate>
        </config>"#,
    )
    .unwrap();

    zip.start_file("Metadata/plate_1.png", options).unwrap();
    zip.write_all(b"\x89PNGfakepngbytes").unwrap();

    zip.finish().unwrap();
    path
}

async fn seed_project(pool: &sqlx::SqlitePool, file_path: &str, mesh_hash: Option<&str>) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO projects (file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes, mesh_hash)
         VALUES (?1, ?2, '2026-01-01T00:00:00Z', 100, ?3)
         RETURNING id",
    )
    .bind(file_path)
    .bind("test.3mf")
    .bind(mesh_hash)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn list_returns_empty_array_when_none_exist() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "GET", "/api/projects", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_returns_seeded_projects() {
    let pool = test_pool().await;
    seed_project(&pool, "/tmp/a.3mf", None).await;

    let (status, body) = send(&pool, "GET", "/api/projects", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["filePath"], "/tmp/a.3mf");
}

#[tokio::test]
async fn get_one_returns_the_project() {
    let pool = test_pool().await;
    let id = seed_project(&pool, "/tmp/detail.3mf", None).await;

    let (status, body) = send(&pool, "GET", &format!("/api/projects/{id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
    assert_eq!(body["filePath"], "/tmp/detail.3mf");
}

#[tokio::test]
async fn get_one_returns_not_found_for_missing_project() {
    let pool = test_pool().await;

    let (status, _) = send(&pool, "GET", "/api/projects/999", None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn plates_reads_real_zip_and_returns_plate_metadata_and_thumbnail() {
    let pool = test_pool().await;
    let fixture_path = write_fixture_3mf("spoolbook_rs_test_projects_plates.3mf");
    let id = seed_project(&pool, fixture_path.to_str().unwrap(), None).await;

    let (status, body) = send(&pool, "GET", &format!("/api/projects/{id}/plates"), None).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let plates = body.as_array().unwrap();
    assert_eq!(plates.len(), 1);
    assert_eq!(plates[0]["platerId"], "1");
    assert_eq!(plates[0]["platerName"], "Plate 1");
    assert!(!plates[0]["thumbnailBytes"].as_str().unwrap().is_empty());

    std::fs::remove_file(fixture_path).ok();
}

#[tokio::test]
async fn plates_returns_not_found_for_missing_project_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "GET", "/api/projects/999/plates", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn version_candidate_matches_by_mesh_hash_first() {
    let pool = test_pool().await;
    seed_project(&pool, "/tmp/old.3mf", Some("hash-abc")).await;
    seed_project(&pool, "/tmp/other.3mf", Some("hash-zzz")).await;

    let (status, body) = send(
        &pool,
        "GET",
        "/api/projects/version-candidate?meshHash=hash-abc&fileName=whatever.3mf",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["filePath"], "/tmp/old.3mf");
}

#[tokio::test]
async fn version_candidate_falls_back_to_filename() {
    let pool = test_pool().await;
    seed_project(&pool, "/tmp/old_name.3mf", None).await;
    sqlx::query("UPDATE projects SET file_name = 'match_me.3mf' WHERE file_path = '/tmp/old_name.3mf'")
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = send(
        &pool,
        "GET",
        "/api/projects/version-candidate?fileName=match_me.3mf",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["fileName"], "match_me.3mf");
}

#[tokio::test]
async fn version_candidate_returns_null_when_nothing_matches() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "GET", "/api/projects/version-candidate?fileName=nope.3mf", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Null);
}

#[tokio::test]
async fn link_version_marks_previous_superseded_and_bumps_version_number() {
    let pool = test_pool().await;
    let old_id = seed_project(&pool, "/tmp/v1.3mf", None).await;
    let new_id = seed_project(&pool, "/tmp/v2.3mf", None).await;

    let (status, body) = send(
        &pool,
        "POST",
        &format!("/api/projects/{new_id}/link-version"),
        Some(serde_json::json!({ "previousVersionProjectId": old_id })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    // list() only returns current versions (link_version_hides_superseded_versions_from_list
    // covers that behavior) -- check the superseded row's own state directly.
    let old_is_current = sqlx::query_scalar::<_, i64>("SELECT is_current_version FROM projects WHERE id = ?1")
        .bind(old_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(old_is_current, 0);

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    let new = all.as_array().unwrap().iter().find(|p| p["id"] == new_id).unwrap();
    assert_eq!(new["isCurrentVersion"], true);
    assert_eq!(new["versionNumber"], 2);
    assert_eq!(new["previousVersionProjectId"], old_id);
}

#[tokio::test]
async fn link_version_hides_superseded_versions_from_list() {
    let pool = test_pool().await;
    let old_id = seed_project(&pool, "/tmp/v1.3mf", None).await;
    let new_id = seed_project(&pool, "/tmp/v2.3mf", None).await;

    send(&pool, "POST", &format!("/api/projects/{new_id}/link-version"), Some(serde_json::json!({ "previousVersionProjectId": old_id }))).await;

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    let ids: Vec<i64> = all.as_array().unwrap().iter().map(|p| p["id"].as_i64().unwrap()).collect();
    assert!(!ids.contains(&old_id), "{ids:?}");
    assert!(ids.contains(&new_id), "{ids:?}");
}

#[tokio::test]
async fn rename_updates_the_display_name() {
    let pool = test_pool().await;
    let id = seed_project(&pool, "/tmp/rename_me.3mf", None).await;

    let (status, body) = send(&pool, "PUT", &format!("/api/projects/{id}"), Some(serde_json::json!({ "fileName": "Better name.3mf" }))).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["project"]["fileName"], "Better name.3mf");

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    assert_eq!(all[0]["fileName"], "Better name.3mf");
}

#[tokio::test]
async fn rename_rejects_a_blank_name() {
    let pool = test_pool().await;
    let id = seed_project(&pool, "/tmp/blank.3mf", None).await;

    let (status, body) = send(&pool, "PUT", &format!("/api/projects/{id}"), Some(serde_json::json!({ "fileName": "   " }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
}

#[tokio::test]
async fn rename_returns_not_found_for_missing_project() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "PUT", "/api/projects/999", Some(serde_json::json!({ "fileName": "x.3mf" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_removes_an_unreferenced_project() {
    let pool = test_pool().await;
    let id = seed_project(&pool, "/tmp/deleteme.3mf", None).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/projects/{id}"), None).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    assert_eq!(all.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn delete_returns_not_found_for_missing_project() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "DELETE", "/api/projects/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn seed_print_referencing_project(pool: &sqlx::SqlitePool, project_id: i64) {
    let filament_id =
        sqlx::query_scalar::<_, i64>("INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black') RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap();
    let spool_id = sqlx::query_scalar::<_, i64>("INSERT INTO spools (filament_id) VALUES (?1) RETURNING id").bind(filament_id).fetch_one(pool).await.unwrap();
    let profile_id = sqlx::query_scalar::<_, i64>("INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, 'Standard', 230) RETURNING id")
        .bind(filament_id)
        .fetch_one(pool)
        .await
        .unwrap();
    let printer_id = sqlx::query_scalar::<_, i64>("INSERT INTO printers (name) VALUES ('Garage P2S') RETURNING id").fetch_one(pool).await.unwrap();

    sqlx::query("INSERT INTO prints (profile_id, spool_id, printer_id, project_id, started_at, status) VALUES (?1, ?2, ?3, ?4, '2026-01-01T00:00:00Z', 'InProgress')")
        .bind(profile_id)
        .bind(spool_id)
        .bind(printer_id)
        .bind(project_id)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn delete_rejects_a_project_referenced_by_a_print() {
    let pool = test_pool().await;
    let id = seed_project(&pool, "/tmp/inuse.3mf", None).await;
    seed_print_referencing_project(&pool, id).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/projects/{id}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
    assert_eq!(body["error"], "has_prints");

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    assert_eq!(all.as_array().unwrap().len(), 1);
}
