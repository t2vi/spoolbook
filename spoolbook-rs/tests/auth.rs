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

// spec: 008/FR-010
#[tokio::test]
async fn setup_status_reports_needs_setup_when_no_users_exist() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "GET", "/api/setup-status", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["needsSetup"], true);
}

// spec: 008/FR-010
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

// spec: 008/FR-010
#[tokio::test]
async fn setup_rejects_a_short_password() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "POST", "/api/setup", None, Some(json!({ "username": "vinny", "password": "short" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "password_too_short");
}

// spec: 008/FR-010
#[tokio::test]
async fn setup_is_rejected_once_a_user_already_exists() {
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(&pool, "POST", "/api/setup", None, Some(json!({ "username": "someone-else", "password": "another-password" }))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "already_set_up");
}

// spec: 008/FR-012
#[tokio::test]
async fn login_fails_with_wrong_password() {
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, set_cookie, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "vinny", "password": "wrong" }))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], false);
    assert!(set_cookie.is_none());
}

// spec: 008/FR-012
#[tokio::test]
async fn login_fails_with_an_unknown_username() {
    let pool = test_pool().await;
    setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(&pool, "POST", "/api/login", None, Some(json!({ "username": "nobody", "password": "anything" }))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], false);
}

// spec: 008/FR-012, 008/FR-001
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

// spec: 008/FR-014
#[tokio::test]
async fn me_reports_unauthenticated_with_no_cookie() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "GET", "/api/me", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["authenticated"], false);
}

// spec: 008/FR-014
#[tokio::test]
async fn me_reports_unauthenticated_with_a_garbage_cookie() {
    let pool = test_pool().await;
    let (_, _, body) = send(&pool, "GET", "/api/me", Some("spoolbook_session=not-a-real-token"), None).await;
    assert_eq!(body["authenticated"], false);
}

// spec: 008/FR-013
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

// spec: 008/FR-001
#[tokio::test]
async fn a_gated_route_rejects_a_request_with_no_cookie() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "PUT", "/api/account", None, Some(json!({ "currentPassword": "x" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["authenticated"], false);
}

// spec: 008/FR-001
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
// Every read route in the app — a new one must be added here (or to the open list below).
const READ_ROUTES: &[&str] = &[
    "/api/dashboard", "/api/export", "/api/filament-colors", "/api/filaments", "/api/filaments/1", "/api/filaments/all",
    "/api/printers", "/api/printers/1", "/api/printers/1/live", "/api/prints", "/api/prints/1", "/api/prints/1/hourly-weather",
    "/api/prints/1/readings", "/api/prints/inventory", "/api/prints/job-match?printerId=1&startedAt=2026-01-01T00:00:00Z",
    "/api/prints/recommend-profile?projectId=1", "/api/profiles?filamentId=1", "/api/profiles/1", "/api/profiles/field-spec",
    "/api/profiles/inventory", "/api/profiles/system-presets", "/api/projects", "/api/projects/1", "/api/projects/1/plates",
    "/api/projects/version-candidate?meshHash=x&fileName=y", "/api/settings", "/api/spools", "/api/spools/1",
    "/api/auth/google/config",
];

// spec: 008/FR-001
#[tokio::test]
async fn read_routes_require_a_session() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    for uri in READ_ROUTES {
        let (status, _, body) = send(&pool, "GET", uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} without a session");
        assert_eq!(body["authenticated"], false, "{uri}");
    }
    let (status, _, _) = send(&pool, "GET", "/api/filaments", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK, "a signed-in user can read");
}

// spec: 008/FR-001
#[tokio::test]
async fn only_the_sign_in_routes_are_open() {
    let pool = test_pool().await;

    for (method, uri) in [
        ("GET", "/api/setup-status"),
        ("GET", "/api/me"),
        ("GET", "/api/version"),
        ("POST", "/api/logout"),
        ("GET", "/api/auth/google/status"),
        ("GET", "/api/auth/google/login"),
        ("GET", "/api/auth/google/callback"),
    ] {
        let (status, _, body) = send(&pool, method, uri, None, None).await;
        assert_ne!(status, StatusCode::UNAUTHORIZED, "{method} {uri} must stay open: {body:?}");
    }
}

// spec: 008/FR-020
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

// spec: 008/FR-020
#[tokio::test]
async fn account_update_rejects_the_wrong_current_password() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(&pool, "PUT", "/api/account", Some(&cookie), Some(json!({ "currentPassword": "wrong", "newPassword": "whatever12" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "wrong_current_password");
}

// spec: 008/FR-011
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

// spec: 008/FR-011
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

// spec: 008/FR-011
#[tokio::test]
async fn startup_migration_does_nothing_when_no_env_var_is_set() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_password(None);
    let pool = test_pool().await;

    spoolbook_rs::auth::migrate_env_var_admin_if_needed(&pool).await;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}

async fn login_status(pool: &sqlx::SqlitePool, username: &str, password: &str) -> StatusCode {
    send(pool, "POST", "/api/login", None, Some(json!({ "username": username, "password": password }))).await.0
}

// spec: 008/FR-021
#[tokio::test]
async fn account_update_rejects_a_short_new_password_and_changes_nothing() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(
        &pool,
        "PUT",
        "/api/account",
        Some(&cookie),
        Some(json!({ "currentPassword": "correct-horse-battery", "newUsername": "renamed", "newPassword": "short" })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
    assert_eq!(body["error"], "password_too_short");
    assert_eq!(login_status(&pool, "vinny", "correct-horse-battery").await, StatusCode::OK, "username and password unchanged");
}

// spec: 008/FR-021
#[tokio::test]
async fn account_update_rejects_a_taken_username_and_changes_nothing() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;
    sqlx::query("INSERT INTO users (username, password_hash, role) VALUES ('taken', 'x', 'Admin')").execute(&pool).await.unwrap();

    let (status, _, body) = send(
        &pool,
        "PUT",
        "/api/account",
        Some(&cookie),
        Some(json!({ "currentPassword": "correct-horse-battery", "newUsername": "taken", "newPassword": "a-new-password" })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
    assert_eq!(body["error"], "username_taken");
    assert_eq!(login_status(&pool, "vinny", "correct-horse-battery").await, StatusCode::OK, "username and password unchanged");
}

// spec: 008/FR-020, 008/FR-021
#[tokio::test]
async fn account_update_accepts_the_users_own_current_username() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) =
        send(&pool, "PUT", "/api/account", Some(&cookie), Some(json!({ "currentPassword": "correct-horse-battery", "newUsername": "vinny" }))).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
}

async fn session_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sessions").fetch_one(pool).await.unwrap()
}

async fn login(pool: &sqlx::SqlitePool, username: &str, password: &str) -> (StatusCode, Option<String>, Value) {
    send(pool, "POST", "/api/login", None, Some(json!({ "username": username, "password": password }))).await
}

async fn signed_in(pool: &sqlx::SqlitePool, cookie: &str) -> bool {
    send(pool, "GET", "/api/me", Some(cookie), None).await.2["authenticated"] == true
}

// spec: 008/FR-015
#[tokio::test]
async fn creating_a_session_deletes_expired_ones() {
    let pool = test_pool().await;
    setup_user(&pool, "admin", "correct-horse").await;
    let user_id: i64 = sqlx::query_scalar("SELECT id FROM users").fetch_one(&pool).await.unwrap();
    let past = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES ('stale-1', ?1, ?2), ('stale-2', ?1, ?2)").bind(user_id).bind(&past).execute(&pool).await.unwrap();
    assert_eq!(session_count(&pool).await, 3);

    let (status, _, _) = login(&pool, "admin", "correct-horse").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(session_count(&pool).await, 2, "both expired rows gone; the setup session and the new one stay");
}

// spec: 008/FR-016
#[tokio::test]
async fn sign_in_is_throttled_after_five_failures_and_a_success_clears_it() {
    let pool = test_pool().await;
    setup_user(&pool, "admin", "correct-horse").await;

    for _ in 0..4 {
        assert_eq!(login(&pool, "admin", "wrong").await.0, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(login(&pool, "admin", "correct-horse").await.0, StatusCode::OK, "4 failures don't lock");
    for _ in 0..5 {
        assert_eq!(login(&pool, "admin", "wrong").await.0, StatusCode::UNAUTHORIZED, "the success cleared the earlier 4");
    }

    let (status, set_cookie, body) = login(&pool, "admin", "correct-horse").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"], "too_many_attempts");
    assert!(set_cookie.is_none(), "no session while throttled");

    // Once the failures are 15 minutes old, the right password works again.
    sqlx::query("UPDATE login_attempts SET attempted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-16 minutes')").execute(&pool).await.unwrap();
    assert_eq!(login(&pool, "admin", "correct-horse").await.0, StatusCode::OK);
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM login_attempts").fetch_one(&pool).await.unwrap();
    assert_eq!(left, 0);
}

// spec: 008/FR-016
#[tokio::test]
async fn unknown_usernames_are_throttled_too() {
    let pool = test_pool().await;
    setup_user(&pool, "admin", "correct-horse").await;
    for _ in 0..5 {
        assert_eq!(login(&pool, "ghost", "x").await.0, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(login(&pool, "ghost", "x").await.0, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(login(&pool, "admin", "correct-horse").await.0, StatusCode::OK, "other usernames are unaffected");
}

// spec: 008/FR-022
#[tokio::test]
async fn changing_the_password_ends_every_other_session() {
    let pool = test_pool().await;
    let here = setup_user(&pool, "admin", "correct-horse").await;
    let elsewhere = extract_cookie_pair(&login(&pool, "admin", "correct-horse").await.1.unwrap()).to_string();

    let (status, _, _) = send(&pool, "PUT", "/api/account", Some(&here), Some(json!({ "currentPassword": "correct-horse", "newUsername": "boss" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(signed_in(&pool, &elsewhere).await, "a username change ends nothing");

    let (status, _, _) = send(&pool, "PUT", "/api/account", Some(&here), Some(json!({ "currentPassword": "correct-horse", "newPassword": "battery-staple" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(signed_in(&pool, &here).await, "the session that changed it stays");
    assert!(!signed_in(&pool, &elsewhere).await, "every other session ends");
}
