mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use std::io::{Read, Write};
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
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

// Real .3mf-shaped zip with a project_settings.config entry, matching the fixture shape
// tests/projects.rs and Spoolbook.Desktop.Tests use.
fn write_fixture_3mf(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("3D/3dmodel.model", options).unwrap();
    zip.write_all(b"<mesh>fake geometry</mesh>").unwrap();

    zip.start_file("Metadata/project_settings.config", options).unwrap();
    zip.write_all(br#"{"nozzle_temperature": ["240", "240"], "layer_height": "0.2"}"#).unwrap();

    zip.finish().unwrap();
    path
}

fn write_fixture_3mf_without_settings(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("3D/3dmodel.model", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(b"<mesh>fake geometry</mesh>").unwrap();
    zip.finish().unwrap();
    path
}

async fn seed_filament(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_profile(pool: &sqlx::SqlitePool, filament_id: i64, nozzle_temp_c: i64) -> i64 {
    sqlx::query_scalar::<_, i64>("INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, 'Test profile', ?2) RETURNING id")
        .bind(filament_id)
        .bind(nozzle_temp_c)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_project(pool: &sqlx::SqlitePool, file_path: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO projects (file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes)
         VALUES (?1, 'test.3mf', '2026-01-01T00:00:00Z', 100) RETURNING id",
    )
    .bind(file_path)
    .fetch_one(pool)
    .await
    .unwrap()
}

// Stands in for the standalone slicer-service — echoes back whatever bytes it received in the
// "project" multipart part, so the success test can prove the patched settings actually made it
// through the wire, not just that *a* response came back. Bound to 127.0.0.1:0 (OS-assigned
// port) so tests can run concurrently.
async fn spawn_echo_slicer_service() -> String {
    use axum::extract::Multipart;
    use tokio::net::TcpListener;

    async fn slice(mut multipart: Multipart) -> Vec<u8> {
        while let Ok(Some(field)) = multipart.next_field().await {
            if field.name() == Some("project") {
                return field.bytes().await.unwrap().to_vec();
            }
        }
        Vec::new()
    }

    let app = axum::Router::new().route("/slice", axum::routing::post(slice));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

// RESLICE_SERVICE_URL is process-global env state; every test in this file that touches it
// holds this lock for the duration of its request so the two such tests can't interleave
// (tests in other files run in separate processes, so they're unaffected).
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn spawn_failing_slicer_service() -> String {
    use tokio::net::TcpListener;

    async fn slice() -> (StatusCode, &'static str) {
        (StatusCode::INTERNAL_SERVER_ERROR, "slicer crashed")
    }

    let app = axum::Router::new().route("/slice", axum::routing::post(slice));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

#[tokio::test]
async fn reslice_returns_not_found_for_a_missing_project() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let profile_id = seed_profile(&pool, filament_id, 240).await;

    let (status, body) = send(&pool, "POST", "/api/projects/999/reslice", Some(json!({ "profileId": profile_id }))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[tokio::test]
async fn reslice_returns_not_found_for_a_missing_profile() {
    let pool = test_pool().await;
    let fixture_path = write_fixture_3mf("spoolbook_rs_test_reslice_missing_profile.3mf");
    let project_id = seed_project(&pool, fixture_path.to_str().unwrap()).await;

    let (status, body) = send(&pool, "POST", &format!("/api/projects/{project_id}/reslice"), Some(json!({ "profileId": 999 }))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");

    std::fs::remove_file(fixture_path).ok();
}

#[tokio::test]
async fn reslice_reports_an_error_when_the_project_has_no_settings_entry() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let profile_id = seed_profile(&pool, filament_id, 250).await;
    let fixture_path = write_fixture_3mf_without_settings("spoolbook_rs_test_reslice_no_settings.3mf");
    let project_id = seed_project(&pool, fixture_path.to_str().unwrap()).await;

    let (status, body) = send(&pool, "POST", &format!("/api/projects/{project_id}/reslice"), Some(json!({ "profileId": profile_id }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().starts_with("Couldn't prepare project for re-slicing"), "{body:?}");

    std::fs::remove_file(fixture_path).ok();
}

#[tokio::test]
async fn reslice_reports_an_error_when_the_slicer_service_fails() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("RESLICE_SERVICE_URL", spawn_failing_slicer_service().await) };

    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let profile_id = seed_profile(&pool, filament_id, 250).await;
    let fixture_path = write_fixture_3mf("spoolbook_rs_test_reslice_slicer_failure.3mf");
    let project_id = seed_project(&pool, fixture_path.to_str().unwrap()).await;

    let (status, body) = send(&pool, "POST", &format!("/api/projects/{project_id}/reslice"), Some(json!({ "profileId": profile_id }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().starts_with("Re-slice failed"), "{body:?}");

    std::fs::remove_file(fixture_path).ok();
}

#[tokio::test]
async fn reslice_patches_settings_and_saves_the_sliced_result() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("RESLICE_SERVICE_URL", spawn_echo_slicer_service().await) };

    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let profile_id = seed_profile(&pool, filament_id, 260).await;
    let fixture_path = write_fixture_3mf("spoolbook_rs_test_reslice_success.3mf");
    let project_id = seed_project(&pool, fixture_path.to_str().unwrap()).await;

    let (status, body) = send(&pool, "POST", &format!("/api/projects/{project_id}/reslice"), Some(json!({ "profileId": profile_id }))).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["project"]["fileName"], "test.3mf");

    let saved_path = body["project"]["filePath"].as_str().unwrap();
    let saved = std::fs::File::open(saved_path).unwrap();
    let mut saved_zip = zip::ZipArchive::new(saved).unwrap();
    let mut settings_json = String::new();
    saved_zip.by_name("Metadata/project_settings.config").unwrap().read_to_string(&mut settings_json).unwrap();
    let settings: Value = serde_json::from_str(&settings_json).unwrap();

    assert_eq!(settings["nozzle_temperature"][0], "260", "patched value made it through the echo round-trip");
    assert_eq!(settings["nozzle_temperature"][1], "240", "second filament slot untouched");
    assert_eq!(settings["layer_height"], "0.2", "key not owned by PrintProfile survives untouched");

    std::fs::remove_file(fixture_path).ok();
}

// A real slicer's output is never byte-identical run to run, so project_upload's content-hash
// dedup alone can never catch "re-sliced the same project again" -- reslicing.rs chains the
// result onto the source project instead (docs/adr/0031's sibling gap, filed as issue #114).
#[tokio::test]
async fn reslice_chains_the_result_as_a_new_version_of_the_source_project() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("RESLICE_SERVICE_URL", spawn_echo_slicer_service().await) };

    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let profile_id = seed_profile(&pool, filament_id, 260).await;
    let fixture_path = write_fixture_3mf("spoolbook_rs_test_reslice_chains_version.3mf");
    let project_id = seed_project(&pool, fixture_path.to_str().unwrap()).await;

    let (status, body) = send(&pool, "POST", &format!("/api/projects/{project_id}/reslice"), Some(json!({ "profileId": profile_id }))).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let new_id = body["project"]["id"].as_i64().unwrap();
    assert_ne!(new_id, project_id, "reslicing patches settings, so the output is never byte-identical to the source");
    assert_eq!(body["project"]["previousVersionProjectId"], project_id);
    assert_eq!(body["project"]["versionNumber"], 2);
    assert_eq!(body["project"]["isCurrentVersion"], true);

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    let ids: Vec<i64> = all.as_array().unwrap().iter().map(|p| p["id"].as_i64().unwrap()).collect();
    assert!(!ids.contains(&project_id), "superseded source shouldn't clutter the list: {ids:?}");
    assert!(ids.contains(&new_id), "{ids:?}");

    std::fs::remove_file(fixture_path).ok();
}
