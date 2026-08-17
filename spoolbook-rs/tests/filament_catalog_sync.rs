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

// The published catalog is PascalCase (verified against the live feed at
// raw.githubusercontent.com/t2vi/spoolbook-filament-sync/main/data/filament-catalog.json).
const SAMPLE_CATALOG: &str = r##"[
    { "Brand": "Bambu Lab", "Material": "PLA", "Variant": "Basic", "Color": "Jade White", "Hex": "#FFFFFF" },
    { "Brand": "Bambu Lab", "Material": "PLA", "Variant": "Basic", "Color": "Beige", "Hex": "#F5F5DC" }
]"##;

#[test]
fn parse_catalog_reads_the_published_pascal_case_shape() {
    let entries = spoolbook_rs::filament_catalog_sync::parse_catalog(SAMPLE_CATALOG);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].brand, "Bambu Lab");
    assert_eq!(entries[0].variant.as_deref(), Some("Basic"));
    assert_eq!(entries[0].color, "Jade White");
    assert_eq!(entries[0].hex.as_deref(), Some("#FFFFFF"));
}

#[test]
fn parse_catalog_returns_empty_on_garbage_input() {
    assert_eq!(spoolbook_rs::filament_catalog_sync::parse_catalog("not json").len(), 0);
}

#[tokio::test]
async fn import_many_counts_added_and_skipped_and_registers_colors() {
    let pool = test_pool().await;
    // Pre-seed one entry identical to the first catalog row so it's skipped as a duplicate.
    send(&pool, "POST", "/api/filaments", Some(json!({
        "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Jade White"
    }))).await;

    let entries = spoolbook_rs::filament_catalog_sync::parse_catalog(SAMPLE_CATALOG);
    let (added, skipped) = spoolbook_rs::filament_catalog_sync::import_many(&pool, &entries).await;

    assert_eq!(added, 1, "Jade White already existed, only Beige is new");
    assert_eq!(skipped, 1);

    let (_, all) = send(&pool, "GET", "/api/filaments/all", None).await;
    assert_eq!(all.as_array().unwrap().len(), 2);

    let (_, colors) = send(&pool, "GET", "/api/filament-colors", None).await;
    let beige = colors.as_array().unwrap().iter().find(|c| c["name"] == "Beige").unwrap();
    assert_eq!(beige["hex"], "#F5F5DC");
}

#[tokio::test]
async fn import_many_skips_entries_that_fail_validation() {
    let pool = test_pool().await;
    let entries = spoolbook_rs::filament_catalog_sync::parse_catalog(
        r#"[{ "Brand": "", "Material": "PLA", "Variant": null, "Color": "Black" }]"#,
    );

    let (added, skipped) = spoolbook_rs::filament_catalog_sync::import_many(&pool, &entries).await;

    assert_eq!(added, 0);
    assert_eq!(skipped, 1);
}
