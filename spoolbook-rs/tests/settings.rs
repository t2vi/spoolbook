mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
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
                .header("cookie", common::auth_cookie_header())
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

#[tokio::test]
async fn get_creates_the_row_lazily_and_returns_defaults() {
    let pool = test_pool().await;

    let (status, body) = send(&pool, "GET", "/api/settings", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["additionalFilamentSourceUrls"], Value::Null);
    assert_eq!(body["lastFilamentSyncAt"], Value::Null);
    assert_eq!(
        body["catalogUrl"],
        "https://raw.githubusercontent.com/t2vi/spoolbook-filament-sync/main/data/filament-catalog.json"
    );
}

#[tokio::test]
async fn post_persists_additional_source_urls() {
    let pool = test_pool().await;

    let (status, body) = send(
        &pool,
        "POST",
        "/api/settings",
        Some(json!({ "additionalFilamentSourceUrls": "https://example.com/catalog.json" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    let (_, get_body) = send(&pool, "GET", "/api/settings", None).await;
    assert_eq!(get_body["additionalFilamentSourceUrls"], "https://example.com/catalog.json");
}

#[tokio::test]
async fn post_blanks_out_whitespace_only_input() {
    let pool = test_pool().await;
    send(&pool, "POST", "/api/settings", Some(json!({ "additionalFilamentSourceUrls": "  " }))).await;

    let (_, get_body) = send(&pool, "GET", "/api/settings", None).await;
    assert_eq!(get_body["additionalFilamentSourceUrls"], Value::Null);
}
