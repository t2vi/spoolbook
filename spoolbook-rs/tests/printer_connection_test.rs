mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn send(uri: &str, body: Value) -> (StatusCode, Value) {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let response = spoolbook_rs::app(pool)
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

// Nothing listens on 127.0.0.1:8883 in this test environment — the connection is refused almost
// immediately (no real broker to test against, same boundary as the rest of this MQTT tier), so
// this proves the plumbing (wire the request through, surface a non-panicking error) without
// waiting out the full 6s timeout.
#[tokio::test]
async fn test_reports_failure_when_nothing_is_listening() {
    let (status, body) = send("/api/printers/test", json!({ "ipAddress": "127.0.0.1", "accessCode": "12345678" })).await;

    assert_eq!(status, StatusCode::OK, "the HTTP call itself succeeds even though the MQTT connect fails");
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().len() > 0);
}
