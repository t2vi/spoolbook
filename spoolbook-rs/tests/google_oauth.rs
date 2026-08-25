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
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
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

async fn location_of(pool: &sqlx::SqlitePool, uri: &str, cookie: Option<&str>) -> String {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let response = spoolbook_rs::app(pool.clone()).oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    response.headers().get(header::LOCATION).unwrap().to_str().unwrap().to_string()
}

fn query_param<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    let (_, query) = url.split_once('?')?;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then_some(v)
    })
}

#[tokio::test]
async fn status_reports_not_configured_by_default() {
    let pool = test_pool().await;
    let (status, _, body) = send(&pool, "GET", "/api/auth/google/status", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["configured"], false);
}

#[tokio::test]
async fn config_endpoint_is_gated() {
    let pool = test_pool().await;
    let (status, _, _) = send(&pool, "GET", "/api/auth/google/config", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = send(&pool, "PUT", "/api/auth/google/config", None, Some(json!({ "clientId": "x", "redirectUri": "y" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn saving_config_makes_status_report_configured_and_hides_the_secret_on_read() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    let (status, _, body) = send(
        &pool,
        "PUT",
        "/api/auth/google/config",
        Some(&cookie),
        Some(json!({ "clientId": "abc.apps.googleusercontent.com", "clientSecret": "shh", "redirectUri": "https://spoolbook.example.com/api/auth/google/callback" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    let (status, _, body) = send(&pool, "GET", "/api/auth/google/status", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["configured"], true);

    let (status, _, body) = send(&pool, "GET", "/api/auth/google/config", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["clientId"], "abc.apps.googleusercontent.com");
    assert_eq!(body["redirectUri"], "https://spoolbook.example.com/api/auth/google/callback");
    assert_eq!(body["secretSet"], true);
    assert!(body.get("clientSecret").is_none(), "secret should never round-trip in a GET: {body:?}");
}

#[tokio::test]
async fn saving_config_with_a_blank_secret_keeps_the_existing_one() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;

    send(&pool, "PUT", "/api/auth/google/config", Some(&cookie), Some(json!({ "clientId": "id1", "clientSecret": "secret1", "redirectUri": "https://a/cb" }))).await;
    let (status, _, body) = send(&pool, "PUT", "/api/auth/google/config", Some(&cookie), Some(json!({ "clientId": "id2", "clientSecret": "", "redirectUri": "https://b/cb" }))).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    let (_, _, body) = send(&pool, "GET", "/api/auth/google/config", Some(&cookie), None).await;
    assert_eq!(body["clientId"], "id2");
    assert_eq!(body["redirectUri"], "https://b/cb");
    assert_eq!(body["secretSet"], true, "blank secret on save should not clear the existing one");
}

#[tokio::test]
async fn login_redirect_requires_configuration() {
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;
    let (status, _, _) = send(&pool, "GET", "/api/auth/google/login", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn configure_google(pool: &sqlx::SqlitePool, cookie: &str, token_url: &str, userinfo_url: &str) {
    unsafe {
        std::env::set_var("GOOGLE_TOKEN_URL", token_url);
        std::env::set_var("GOOGLE_USERINFO_URL", userinfo_url);
    }
    let (status, _, body) = send(
        pool,
        "PUT",
        "/api/auth/google/config",
        Some(cookie),
        Some(json!({ "clientId": "client-id", "clientSecret": "client-secret", "redirectUri": "https://spoolbook.example.com/api/auth/google/callback" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
}

// GOOGLE_TOKEN_URL/GOOGLE_USERINFO_URL are process-global env state; every test in this file that
// touches them holds this lock for its whole request sequence (tests in other files run in
// separate processes, unaffected).
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// Stands in for Google's token + userinfo endpoints so the callback handler can be tested without
// a real network call — same local-mock-server pattern tests/reslicing.rs uses for the slicer
// service. `sub` is baked into the mock so each test controls which "Google account" signs in.
async fn spawn_mock_google(sub: &str) -> (String, String) {
    use axum::Json;
    use axum::extract::State;
    use tokio::net::TcpListener;

    #[derive(Clone)]
    struct MockState {
        sub: String,
    }

    async fn token() -> Json<serde_json::Value> {
        Json(json!({ "access_token": "mock-access-token" }))
    }

    async fn userinfo(State(state): State<MockState>) -> Json<serde_json::Value> {
        Json(json!({ "sub": state.sub, "email": format!("{}@example.com", state.sub) }))
    }

    let state = MockState { sub: sub.to_string() };
    let app = axum::Router::new()
        .route("/token", axum::routing::post(token))
        .route("/userinfo", axum::routing::get(userinfo))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/token"), format!("http://{addr}/userinfo"))
}

#[tokio::test]
async fn link_flow_attaches_google_sub_to_the_authenticated_admin_and_me_reports_it() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;
    let (token_url, userinfo_url) = spawn_mock_google("google-sub-1").await;
    configure_google(&pool, &cookie, &token_url, &userinfo_url).await;

    let login_redirect = location_of(&pool, "/api/auth/google/login", Some(&cookie)).await;
    assert!(login_redirect.starts_with("https://accounts.google.com/"), "{login_redirect}");
    let state = query_param(&login_redirect, "state").expect("authorize URL should carry a state param");

    let (status, set_cookie, _) = send(&pool, "GET", &format!("/api/auth/google/callback?code=fake-code&state={state}"), Some(&cookie), None).await;
    assert!(status.is_redirection(), "{status}");
    assert!(set_cookie.is_none(), "linking shouldn't rotate the admin's existing session cookie");

    let (_, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(body["googleLinked"], true);
}

#[tokio::test]
async fn sign_in_flow_succeeds_when_the_google_sub_is_already_linked() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = test_pool().await;
    let admin_cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;
    let (token_url, userinfo_url) = spawn_mock_google("google-sub-2").await;
    configure_google(&pool, &admin_cookie, &token_url, &userinfo_url).await;

    // Link first, as the authenticated admin.
    let login_redirect = location_of(&pool, "/api/auth/google/login", Some(&admin_cookie)).await;
    let state = query_param(&login_redirect, "state").unwrap();
    send(&pool, "GET", &format!("/api/auth/google/callback?code=fake-code&state={state}"), Some(&admin_cookie), None).await;

    // Now sign in fresh, with no cookie at all -- should succeed and set a real session cookie.
    let login_redirect = location_of(&pool, "/api/auth/google/login", None).await;
    let state = query_param(&login_redirect, "state").unwrap();
    let (status, set_cookie, _) = send(&pool, "GET", &format!("/api/auth/google/callback?code=fake-code&state={state}"), None, None).await;
    assert!(status.is_redirection(), "{status}");
    let cookie = extract_cookie_pair(&set_cookie.expect("sign-in should set a session cookie")).to_string();

    let (_, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(body["authenticated"], true);
}

#[tokio::test]
async fn sign_in_flow_rejects_an_unlinked_google_account_without_creating_a_user() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = test_pool().await;
    let admin_cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;
    let (token_url, userinfo_url) = spawn_mock_google("google-sub-never-linked").await;
    configure_google(&pool, &admin_cookie, &token_url, &userinfo_url).await;

    let login_redirect = location_of(&pool, "/api/auth/google/login", None).await;
    let state = query_param(&login_redirect, "state").unwrap();
    let (status, set_cookie, _) = send(&pool, "GET", &format!("/api/auth/google/callback?code=fake-code&state={state}"), None, None).await;
    assert!(status.is_redirection(), "{status}");
    assert!(set_cookie.is_none(), "an unknown Google account must not get a session");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1, "no second user should have been auto-provisioned");
}

#[tokio::test]
async fn callback_rejects_an_unknown_or_reused_state() {
    let pool = test_pool().await;
    let (status, _, _) = send(&pool, "GET", "/api/auth/google/callback?code=fake-code&state=never-issued", None, None).await;
    assert!(status.is_redirection() || status == StatusCode::BAD_REQUEST, "{status}");
}

#[tokio::test]
async fn unlink_clears_google_sub() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = test_pool().await;
    let cookie = setup_user(&pool, "vinny", "correct-horse-battery").await;
    let (token_url, userinfo_url) = spawn_mock_google("google-sub-3").await;
    configure_google(&pool, &cookie, &token_url, &userinfo_url).await;

    let login_redirect = location_of(&pool, "/api/auth/google/login", Some(&cookie)).await;
    let state = query_param(&login_redirect, "state").unwrap();
    send(&pool, "GET", &format!("/api/auth/google/callback?code=fake-code&state={state}"), Some(&cookie), None).await;
    let (_, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(body["googleLinked"], true);

    let (status, _, body) = send(&pool, "DELETE", "/api/auth/google", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    let (_, _, body) = send(&pool, "GET", "/api/me", Some(&cookie), None).await;
    assert_eq!(body["googleLinked"], false);
}

#[tokio::test]
async fn unlink_is_gated() {
    let pool = test_pool().await;
    let (status, _, _) = send(&pool, "DELETE", "/api/auth/google", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
