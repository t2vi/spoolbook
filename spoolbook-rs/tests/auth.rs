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
// other's value (tests in other files run in separate processes, unaffected).
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

async fn setup_user(pool: &sqlx::SqlitePool, username: &str, password: &str) -> String {
    let (status, set_cookie, body) = send(pool, "POST", "/api/setup", None, Some(json!({ "username": username, "password": password }))).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    extract_cookie_pair(&set_cookie.expect("setup should set a cookie")).to_string()
}

#[tokio::test]
async fn setup_status_reports_needs_setup_when_no_users_exist() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "GET", "/api/setup-status", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["needsSetup"], true);
}

#[tokio::test]
async fn setup_creates_the_first_user_and_a_working_session() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (_, _, body) = send(&pool, "GET", "/api/setup-status", None, None).await;
    assert_eq!(body["needsSetup"], false);

    let (status, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["authenticated"], true);
}

#[tokio::test]
async fn setup_rejects_a_short_password() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "POST", "/api/setup", None, Some(json!({ "username": "vinny", "password": "short" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "password_too_short");
}

#[tokio::test]
async fn setup_is_rejected_once_a_user_already_exists() {
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(&pool, "POST", "/api/setup", None, Some(json!({ "username": "someone-else", "password": "another-password" }))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "already_set_up");
}

#[tokio::test]
async fn login_fails_with_wrong_password() {
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, set_cookie, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "vinny", "password": "wrong" }))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], false);
    assert!(set_cookie.is_none());
}

#[tokio::test]
async fn login_fails_with_an_unknown_username() {
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "nobody", "password": "anything" }))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn login_succeeds_and_sets_a_session_cookie_the_gated_routes_accept() {
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, set_cookie, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "vinny", "password": "correct-horse-battery" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    let cookie = extract_cookie_pair(&set_cookie.expect("login should set a cookie")).to_string();

    let (status, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["authenticated"], true);

    // The cookie should also pass a real gated mutating route, not just /api/me.
    let (status, _, body) = send(&pool, "PUT", "/api/account", Some(&cookie), Some(json!({ "currentPassword": "correct-horse-battery" }))).await;
    assert_ne!(status, StatusCode::UNAUTHORIZED, "{body:?}");
}

#[tokio::test]
async fn me_reports_unauthenticated_with_no_cookie() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "GET", "/api/me", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["authenticated"], false);
}

#[tokio::test]
async fn me_reports_unauthenticated_with_a_garbage_cookie() {
    let pool = test_pool().await;
    let (_, _, body) = send(&pool, "GET", "/api/me", Some("spoolbook_session=not-a-real-token"), None).await;
    assert_eq!(body["authenticated"], false);
}

#[tokio::test]
async fn logout_deletes_the_session_so_the_cookie_stops_working() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, set_cookie, body) = send(&pool, "POST", "/api/logout", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    let set_cookie = set_cookie.expect("logout should clear the cookie");
    assert!(set_cookie.contains("Max-Age=0"), "{set_cookie}");

    // Server-side invalidation, not just a client-side clear -- the same cookie value must no
    // longer work even if a client held onto it.
    let (_, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(body["authenticated"], false);
}

#[tokio::test]
async fn a_gated_route_rejects_a_request_with_no_cookie() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "PUT", "/api/account", None, Some(json!({ "currentPassword": "x" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["authenticated"], false);
}

#[tokio::test]
async fn a_gated_route_rejects_an_expired_session() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;
    let token = cookie.strip_prefix("spoolbook_session=").unwrap();

    sqlx::query("UPDATE sessions SET expires_at = '2000-01-01T00:00:00Z' WHERE id = ?1")
        .bind(token)
        .execute(&pool)
        .await
        .unwrap();

    let (status, _, _) = send(&pool, "PUT", "/api/account", Some(&cookie), Some(json!({ "currentPassword": "x" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// Read (GET) routes stay open, matching the original shared-secret model — only mutations are
// gated.
#[tokio::test]
async fn read_routes_stay_open_with_no_cookie() {
    let pool = test_pool().await;
    let (status, _, _) = send(&pool, "GET", "/api/filaments", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn account_update_changes_username_and_password() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(
        &pool,
        "PUT",
        "/api/account",
        Some(&cookie),
        Some(json!({ "currentPassword": "correct-horse-battery", "newUsername": "renamed", "newPassword": "a-new-password" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    // Old password no longer works, new one does, under the new username.
    let (status, _, _) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "renamed", "password": "correct-horse-battery" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, set_cookie, _) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "renamed", "password": "a-new-password" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(set_cookie.is_some());
}

#[tokio::test]
async fn account_update_rejects_the_wrong_current_password() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(&pool, "PUT", "/api/account", Some(&cookie), Some(json!({ "currentPassword": "wrong", "newPassword": "whatever12" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "wrong_current_password");
}

#[tokio::test]
async fn startup_migration_creates_an_admin_user_from_the_env_var_when_users_table_is_empty() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("legacy-shared-secret"));
    let pool = test_pool().await;

    spoolbook_rs::auth::migrate_env_var_admin_if_needed(&pool).await;

    let (status, set_cookie, _) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "admin", "password": "legacy-shared-secret" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(set_cookie.is_some());
}

#[tokio::test]
async fn startup_migration_does_nothing_when_a_user_already_exists() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(Some("legacy-shared-secret"));
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "already-set-up-password").await;

    spoolbook_rs::auth::migrate_env_var_admin_if_needed(&pool).await;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1, "should not have created a second user");
}

#[tokio::test]
async fn startup_migration_does_nothing_when_no_env_var_is_set() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(None);
    let pool = test_pool().await;

    spoolbook_rs::auth::migrate_env_var_admin_if_needed(&pool).await;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}
