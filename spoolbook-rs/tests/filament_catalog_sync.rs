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

// The published catalog is PascalCase (verified against the live feed at
// raw.githubusercontent.com/t2vi/spoolbook-filament-sync/main/data/filament-catalog.json).
const SAMPLE_CATALOG: &str = r##"[
    { "Brand": "Bambu Lab", "Material": "PLA", "Variant": "Basic", "Color": "Jade White", "Hex": "#FFFFFF" },
    { "Brand": "Bambu Lab", "Material": "PLA", "Variant": "Basic", "Color": "Beige", "Hex": "#F5F5DC" }
]"##;

// spec: 007/FR-023
#[test]
fn parse_catalog_reads_the_published_pascal_case_shape() {
    let entries = spoolbook_rs::filament_catalog_sync::parse_catalog(SAMPLE_CATALOG);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].brand, "Bambu Lab");
    assert_eq!(entries[0].variant.as_deref(), Some("Basic"));
    assert_eq!(entries[0].color, "Jade White");
    assert_eq!(entries[0].hex.as_deref(), Some("#FFFFFF"));
}

// spec: 007/FR-023
#[test]
fn parse_catalog_returns_empty_on_garbage_input() {
    assert_eq!(spoolbook_rs::filament_catalog_sync::parse_catalog("not json").len(), 0);
}

// spec: 007/FR-020
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

// spec: 007/FR-022
#[test]
fn should_sync_when_never_synced() {
    let now = chrono::Utc::now();
    assert!(spoolbook_rs::filament_catalog_sync::should_sync(None, now));
}

// spec: 007/FR-022
#[test]
fn should_sync_when_last_sync_over_24h_ago() {
    let now = chrono::Utc::now();
    let last = (now - chrono::Duration::hours(25)).to_rfc3339();
    assert!(spoolbook_rs::filament_catalog_sync::should_sync(Some(&last), now));
}

// spec: 007/FR-022
#[test]
fn should_not_sync_when_last_sync_within_24h() {
    let now = chrono::Utc::now();
    let last = (now - chrono::Duration::hours(1)).to_rfc3339();
    assert!(!spoolbook_rs::filament_catalog_sync::should_sync(Some(&last), now));
}

// spec: 007/FR-020
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

// FILAMENT_CATALOG_URL is process-global; every test that sets it holds this lock throughout.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// Serves `body` with `status` at /catalog.json and returns that URL.
async fn spawn_catalog(status: u16, body: &'static str) -> String {
    let app = axum::Router::new().route(
        "/catalog.json",
        axum::routing::get(move || async move { (StatusCode::from_u16(status).unwrap(), body) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/catalog.json")
}

fn set_catalog_url(url: &str) {
    unsafe { std::env::set_var("FILAMENT_CATALOG_URL", url) };
}

async fn last_sync(pool: &sqlx::SqlitePool) -> Option<String> {
    sqlx::query_scalar("SELECT last_filament_sync_at FROM app_settings WHERE id = 1").fetch_optional(pool).await.unwrap().flatten()
}

const ONE_ENTRY: &str = r##"[{ "Brand": "Bambu Lab", "Material": "PLA", "Variant": "Basic", "Color": "Black", "Hex": "#000000" }]"##;

// spec: 007/FR-021
#[tokio::test]
async fn sync_fails_and_keeps_the_sync_time_when_the_catalog_returns_404() {
    let _guard = ENV_LOCK.lock().await;
    set_catalog_url(&spawn_catalog(404, "not found").await);
    let pool = test_pool().await;

    let (status, body) = send(&pool, "POST", "/api/filaments/sync", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["ok"], false);
    assert_eq!(last_sync(&pool).await, None);
}

// spec: 007/FR-021
#[tokio::test]
async fn sync_fails_when_the_catalog_body_is_not_a_catalog() {
    let _guard = ENV_LOCK.lock().await;
    set_catalog_url(&spawn_catalog(200, "<html>maintenance</html>").await);
    let pool = test_pool().await;

    let (status, body) = send(&pool, "POST", "/api/filaments/sync", None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(last_sync(&pool).await, None);
}

// spec: 007/FR-020, 007/FR-024
#[tokio::test]
async fn sync_imports_the_catalog_and_records_the_sync_time() {
    let _guard = ENV_LOCK.lock().await;
    set_catalog_url(&spawn_catalog(200, ONE_ENTRY).await);
    let pool = test_pool().await;

    let (status, body) = send(&pool, "POST", "/api/filaments/sync", None).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["added"], 1);
    assert!(last_sync(&pool).await.is_some());
}

// spec: 007/FR-020
#[tokio::test]
async fn a_failing_additional_source_is_skipped() {
    let _guard = ENV_LOCK.lock().await;
    set_catalog_url(&spawn_catalog(200, ONE_ENTRY).await);
    let broken = spawn_catalog(500, "boom").await;
    let pool = test_pool().await;
    send(&pool, "POST", "/api/settings", Some(json!({ "additionalFilamentSourceUrls": broken, "latitude": null, "longitude": null }))).await;

    let (status, body) = send(&pool, "POST", "/api/filaments/sync", None).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["added"], 1);
}

// spec: 007/FR-024, 002/FR-010
#[tokio::test]
async fn settings_report_the_overridden_catalog_url() {
    let _guard = ENV_LOCK.lock().await;
    set_catalog_url("http://127.0.0.1:1/override.json");
    let pool = test_pool().await;

    let (_, body) = send(&pool, "GET", "/api/settings", None).await;

    assert_eq!(body["catalogUrl"], "http://127.0.0.1:1/override.json");
}
