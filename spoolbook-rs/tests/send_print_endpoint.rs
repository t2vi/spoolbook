mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use std::io::Write;
use tower::ServiceExt;

// Only the paths that fail *before* any network call (FTP upload / MQTT publish) are testable
// here — there's no real printer in this environment. Matches printer_mqtt.rs's established
// boundary: the wire protocol itself is unverified, exercised as far as it can be deterministically.
async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn send(pool: &sqlx::SqlitePool, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", common::auth_cookie_header())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

fn print_request_body(project_id: i64) -> Value {
    json!({ "projectId": project_id, "platerId": "1", "spoolId": 1, "profileId": 1, "useAms": true, "amsSlot": 0 })
}

async fn seed_printer_missing_details(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("INSERT INTO printers (name) VALUES ('No connection details') RETURNING id").fetch_one(pool).await.unwrap()
}

async fn seed_printer_with_details(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO printers (name, model, ip_address, access_code, serial_number) VALUES ('Garage P2S', 'P2S', '127.0.0.1', '12345678', 'SERIAL1') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fn write_fixture_3mf_without_gcode(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("3D/3dmodel.model", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(b"<mesh>fake geometry</mesh>").unwrap();
    zip.finish().unwrap();
    path
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

#[tokio::test]
async fn start_print_rejects_a_printer_missing_connection_details() {
    let pool = test_pool().await;
    let printer_id = seed_printer_missing_details(&pool).await;

    let (status, body) = send(&pool, &format!("/api/printers/{printer_id}/print"), print_request_body(1)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"], "Printer is missing connection details.");
}

#[tokio::test]
async fn start_print_rejects_a_missing_printer() {
    let pool = test_pool().await;

    let (status, body) = send(&pool, "/api/printers/999/print", print_request_body(1)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"], "Printer is missing connection details.");
}

#[tokio::test]
async fn start_print_returns_not_found_for_a_missing_project() {
    let pool = test_pool().await;
    let printer_id = seed_printer_with_details(&pool).await;

    let (status, body) = send(&pool, &format!("/api/printers/{printer_id}/print"), print_request_body(999)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[tokio::test]
async fn start_print_reports_an_error_when_the_plate_gcode_entry_is_missing() {
    let pool = test_pool().await;
    let printer_id = seed_printer_with_details(&pool).await;
    let fixture_path = write_fixture_3mf_without_gcode("spoolbook_rs_test_send_print_no_gcode.3mf");
    let project_id = seed_project(&pool, fixture_path.to_str().unwrap()).await;

    let (status, body) = send(&pool, &format!("/api/printers/{printer_id}/print"), print_request_body(project_id)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().starts_with("Couldn't find Metadata/plate_1.gcode"), "{body:?}");

    std::fs::remove_file(fixture_path).ok();
}
