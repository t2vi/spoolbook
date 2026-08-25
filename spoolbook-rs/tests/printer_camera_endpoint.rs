mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use spoolbook_rs::printer_camera::CameraStreamStatus;
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn seed_printer(pool: &sqlx::SqlitePool, ip_address: Option<&str>, access_code: Option<&str>) -> i64 {
    sqlx::query_scalar::<_, i64>("INSERT INTO printers (name, ip_address, access_code) VALUES ('Garage P2S', ?1, ?2) RETURNING id")
        .bind(ip_address)
        .bind(access_code)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn camera_stream_returns_not_found_for_a_missing_printer() {
    let pool = test_pool().await;
    let cookie = common::auth_cookie_header(&pool).await;
    let app = spoolbook_rs::app(pool);

    let response = app
        .oneshot(Request::builder().method("GET").uri("/printers/999/camera").header("cookie", cookie).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn camera_stream_returns_not_found_for_a_printer_missing_connection_details() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool, None, None).await;
    let cookie = common::auth_cookie_header(&pool).await;
    let app = spoolbook_rs::app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/printers/{printer_id}/camera"))
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Doesn't wait for any actual frame data (there's no real printer/ffmpeg in this environment) —
// the response headers are already finalized by the time subscribe() returns, before any bytes
// are streamed, so this proves the routing/header wiring without touching the unverifiable part.
#[tokio::test]
async fn camera_stream_responds_with_multipart_headers_for_a_configured_printer() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool, Some("127.0.0.1"), Some("12345678")).await;
    let live_status = spoolbook_rs::printer_mqtt::new_store();
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    let cookie = common::auth_cookie_header(&pool).await;
    let app = spoolbook_rs::app_with_camera(pool, live_status, camera_registry.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/printers/{printer_id}/camera"))
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "multipart/x-mixed-replace; boundary=frame");
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-cache");

    // subscribe() sets Connecting synchronously (and starts the pipeline) before the handler
    // returns — this is the one state-machine transition observable without any real printer.
    let status = spoolbook_rs::printer_camera::status_of(&camera_registry, printer_id).await;
    assert_ne!(status, CameraStreamStatus::NotStarted, "subscribing should have kicked off the pipeline");
}

#[tokio::test]
async fn retry_is_a_no_op_for_a_printer_with_no_broadcaster_yet() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool, Some("127.0.0.1"), Some("12345678")).await;
    let app = spoolbook_rs::app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/printers/{printer_id}/camera/retry"))
                .header("content-type", "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["ok"], true);
}
