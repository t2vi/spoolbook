mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
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

async fn post_multipart(pool: &sqlx::SqlitePool, uri: &str, field_name: &str, filename: &str, content: &[u8]) -> (StatusCode, Value) {
    let boundary = "spoolbook-rs-test-boundary";
    let mut body = Vec::new();
    write!(
        body,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .unwrap();
    body.extend_from_slice(content);
    write!(body, "\r\n--{boundary}--\r\n").unwrap();

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
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

// Minimal real zip with the mesh entry so upsert's compute_mesh_hash succeeds — not a mock,
// same fixture shape tests/projects.rs uses for plate reading.
fn fake_3mf_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("3D/3dmodel.model", options).unwrap();
        zip.write_all(b"<mesh>fake geometry</mesh>").unwrap();
        zip.finish().unwrap();
    }
    buf
}

#[tokio::test]
async fn upload_saves_the_file_and_creates_a_project() {
    let pool = test_pool().await;
    let bytes = fake_3mf_bytes();

    let (status, body) = post_multipart(&pool, "/api/projects/upload", "file", "cool_vase.3mf", &bytes).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["created"], true);
    assert_eq!(body["project"]["fileName"], "cool_vase.3mf");
    assert!(body["project"]["meshHash"].as_str().unwrap().len() > 0);

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    assert_eq!(all.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn upload_the_same_bytes_twice_dedupes_to_one_project() {
    let pool = test_pool().await;
    let bytes = fake_3mf_bytes();

    let (_, first) = post_multipart(&pool, "/api/projects/upload", "file", "a.3mf", &bytes).await;
    let (status, second) = post_multipart(&pool, "/api/projects/upload", "file", "a.3mf", &bytes).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["created"], false);
    assert_eq!(second["project"]["id"], first["project"]["id"]);

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    assert_eq!(all.as_array().unwrap().len(), 1);
}

// Reproduces the real bug: axum's DefaultBodyLimit defaults to 2MB, well below a real sliced
// .3mf's size (embedded gcode + thumbnails). Padding fake_3mf_bytes() past that line without a
// real DefaultBodyLimit::max() layer on the app used to fail the multipart read entirely with a
// generic "Couldn't read the uploaded file." error, not just reject cleanly.
#[tokio::test]
async fn upload_accepts_a_file_larger_than_axums_default_2mb_body_limit() {
    let pool = test_pool().await;
    let mut bytes = fake_3mf_bytes();
    bytes.extend(std::iter::repeat_n(0u8, 3 * 1024 * 1024));

    let (status, body) = post_multipart(&pool, "/api/projects/upload", "file", "big.3mf", &bytes).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn upload_rejects_a_request_with_no_file_field() {
    let pool = test_pool().await;
    let (status, body) = post_multipart(&pool, "/api/projects/upload", "not_file", "a.3mf", b"junk").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "No file provided.");
}

#[tokio::test]
async fn import_url_rejects_a_non_http_url() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "POST", "/api/projects/import-url", Some(json!({ "url": "ftp://example.com/a.3mf" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Enter a valid http(s) URL.");
}

#[tokio::test]
async fn import_url_rejects_a_malformed_url() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "POST", "/api/projects/import-url", Some(json!({ "url": "not a url" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Enter a valid http(s) URL.");
}
