mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    pool
}

async fn send(uri: &str, body: Value) -> (StatusCode, Value) {
    send_on(test_pool().await, uri, body).await
}

async fn send_on(pool: sqlx::SqlitePool, uri: &str, body: Value) -> (StatusCode, Value) {
    let cookie = common::auth_cookie_header(&pool).await;
    let response = spoolbook_rs::app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", cookie)
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

// Nothing listens on 127.0.0.1:8883 in this test environment — the connection is refused almost
// immediately (no real broker to test against, same boundary as the rest of this MQTT tier), so
// this proves the plumbing (wire the request through, surface a non-panicking error) without
// waiting out the full 6s timeout.
// spec: 004/FR-005
#[tokio::test]
async fn test_reports_failure_when_nothing_is_listening() {
    let (status, body) = send("/api/printers/test", json!({ "ipAddress": "127.0.0.1", "accessCode": "12345678" })).await;

    assert_eq!(status, StatusCode::OK, "the HTTP call itself succeeds even though the MQTT connect fails");
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().len() > 0);
}

// spec: 004/FR-005
#[tokio::test]
async fn test_requires_an_access_code_or_a_printer_with_one() {
    let (status, body) = send("/api/printers/test", json!({ "ipAddress": "127.0.0.1" })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Access code is required");
}

// spec: 004/FR-005
#[tokio::test]
async fn test_falls_back_to_the_stored_access_code_of_the_given_printer() {
    let pool = test_pool().await;
    let id: i64 = sqlx::query_scalar("INSERT INTO printers (name, access_code) VALUES ('P2S', '12345678') RETURNING id").fetch_one(&pool).await.unwrap();

    let (status, body) = send_on(pool, "/api/printers/test", json!({ "ipAddress": "127.0.0.1", "accessCode": "", "printerId": id })).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["ok"], false, "got as far as a real connect attempt");
}
