use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

// SPOOLBOOK_ADMIN_PASSWORD is process-global env state; every test that touches it holds this
// lock for its whole request sequence so tests in this file can't interleave and clobber each
// other's password (tests in other files run in separate processes, unaffected).
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn set_password(value: Option<&str>) {
    unsafe {
        match value {
            Some(v) => std::env::set_var("SPOOLBOOK_ADMIN_PASSWORD", v),
            None => std::env::remove_var("SPOOLBOOK_ADMIN_PASSWORD"),
        }
    }
}

async fn send(pool: &sqlx::SqlitePool, method: &str, uri: &str, cookie: Option<&str>, body: Option<Value>) -> (StatusCode, Option<String>, Value) {
    let mut builder = Request::builder().method(method).uri(uri).header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let response = spoolbook_rs::app(pool.clone()).oneshot(builder.body(Body::from(body)).unwrap()).await.unwrap();

    let status = response.status();
    let set_cookie = response.headers().get(header::SET_COOKIE).map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, set_cookie, json)
}

fn extract_cookie_pair(set_cookie: &str) -> &str {
    set_cookie.split(';').next().unwrap()
}

#[tokio::test]
async fn login_fails_with_wrong_password() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("correct-horse"));
    let pool = test_pool().await;

    let (status, set_cookie, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "password": "wrong" }))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], false);
    assert!(set_cookie.is_none());
}

#[tokio::test]
async fn login_fails_when_no_password_is_configured() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(None);
    let pool = test_pool().await;

    let (status, _, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "password": "anything" }))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn login_succeeds_and_sets_a_cookie_the_gated_routes_accept() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("correct-horse"));
    let pool = test_pool().await;

    let (status, set_cookie, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "password": "correct-horse" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    let cookie = extract_cookie_pair(&set_cookie.expect("login should set a cookie")).to_string();

    let (status, _, _) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);

    let (_, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(body["authenticated"], true);

    // The cookie should also pass a real gated mutating route, not just /api/me.
    let (status, _, body) = send(&pool, "POST", "/api/settings", Some(&cookie), Some(json!({}))).await;
    assert_ne!(status, StatusCode::UNAUTHORIZED, "{body:?}");
}

#[tokio::test]
async fn me_reports_unauthenticated_with_no_cookie() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("correct-horse"));
    let pool = test_pool().await;

    let (status, _, body) = send(&pool, "GET", "/api/me", None, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["authenticated"], false);
}

#[tokio::test]
async fn me_reports_unauthenticated_with_a_garbage_cookie() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("correct-horse"));
    let pool = test_pool().await;

    let (_, _, body) = send(&pool, "GET", "/api/me", Some("spoolbook_editor=not-the-real-token"), None).await;

    assert_eq!(body["authenticated"], false);
}

#[tokio::test]
async fn logout_clears_the_cookie() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("correct-horse"));
    let pool = test_pool().await;

    let (_, _, _) = send(&pool, "POST", "/api/login", None, Some(json!({ "password": "correct-horse" }))).await;
    let (status, set_cookie, body) = send(&pool, "POST", "/api/logout", None, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    let set_cookie = set_cookie.expect("logout should clear the cookie");
    assert!(set_cookie.contains("Max-Age=0"), "{set_cookie}");
}

#[tokio::test]
async fn a_gated_route_rejects_a_request_with_no_cookie() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("correct-horse"));
    let pool = test_pool().await;

    let (status, _, body) = send(&pool, "POST", "/api/settings", None, Some(json!({}))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["authenticated"], false);
}

#[tokio::test]
async fn a_gated_route_rejects_a_stale_cookie_after_the_password_changes() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("old-password"));
    let pool = test_pool().await;
    let (_, set_cookie, _) = send(&pool, "POST", "/api/login", None, Some(json!({ "password": "old-password" }))).await;
    let cookie = extract_cookie_pair(&set_cookie.unwrap()).to_string();

    set_password(Some("new-password"));
    let (status, _, _) = send(&pool, "POST", "/api/settings", Some(&cookie), Some(json!({}))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// Read (GET) routes stay open, matching the C# original — only mutations are gated.
#[tokio::test]
async fn read_routes_stay_open_with_no_cookie() {
    let pool = test_pool().await;

    let (status, _, _) = send(&pool, "GET", "/api/filaments", None, None).await;
    assert_eq!(status, StatusCode::OK);
}
