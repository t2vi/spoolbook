// Real user accounts (users + sessions tables) replaced the single shared-secret HMAC cookie
// this crate used to run on. Session tokens are opaque random values looked up server-side
// (sessions.id), not a recomputable proof the way the old HMAC(password, "editor") cookie was —
// that trick only worked because the shared password doubled as both credential and signing key,
// which breaks once login means "verify against an argon2 hash in a users row." See
// docs/adr/0027 for the full reasoning.
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng as PwOsRng};
use argon2::Argon2;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::{HeaderMap, StatusCode, header, request::Parts};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::{get, post, put}};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;

const COOKIE_NAME: &str = "spoolbook_session";
const SESSION_TTL_DAYS: i64 = 90;

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut PwOsRng);
    Argon2::default().hash_password(password.as_bytes(), &salt).expect("hashing failed").to_string()
}

fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else { return false };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

pub(crate) fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn get_cookie<'h>(headers: &'h HeaderMap, name: &str) -> Option<&'h str> {
    headers.get(header::COOKIE)?.to_str().ok()?.split(';').map(str::trim).find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == name).then_some(v)
    })
}

pub(crate) fn set_cookie_header(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, format!("{COOKIE_NAME}={value}; Path=/; HttpOnly; SameSite=Lax").parse().unwrap());
    headers
}

pub(crate) async fn create_session(pool: &SqlitePool, user_id: i64) -> String {
    let token = generate_session_token();
    let expires_at = (Utc::now() + Duration::days(SESSION_TTL_DAYS)).to_rfc3339();
    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES (?1, ?2, ?3)")
        .bind(&token)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool)
        .await
        .expect("insert failed");
    token
}

// Shared by the me() handler and the Editor extractor so "am I logged in" and "am I allowed to
// mutate" can never disagree. Admin-only role check is real code now (not a placeholder) even
// though every user is Admin today -- adding a second role later is a data change, not a rewrite.
pub(crate) async fn current_user_id(pool: &SqlitePool, headers: &HeaderMap) -> Option<i64> {
    let token = get_cookie(headers, COOKIE_NAME)?;
    let row = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT sessions.user_id, sessions.expires_at, users.role FROM sessions JOIN users ON users.id = sessions.user_id WHERE sessions.id = ?1",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .expect("query failed")?;
    let (user_id, expires_at, role) = row;
    if role != "Admin" {
        return None;
    }
    let expires: DateTime<Utc> = DateTime::parse_from_rfc3339(&expires_at).ok()?.with_timezone(&Utc);
    (Utc::now() <= expires).then_some(user_id)
}

pub struct Editor {
    pub user_id: i64,
}

pub struct AuthError;

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, Json(json!({ "authenticated": false }))).into_response()
    }
}

impl<S> FromRequestParts<S> for Editor
where
    S: Send + Sync,
    SqlitePool: axum::extract::FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = SqlitePool::from_ref(state);
        current_user_id(&pool, &parts.headers).await.map(|user_id| Editor { user_id }).ok_or(AuthError)
    }
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/setup-status", get(setup_status))
        .route("/api/setup", post(setup))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me))
        .route("/api/account", put(update_account))
}

async fn user_count(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await.expect("query failed")
}

async fn setup_status(axum::extract::State(pool): axum::extract::State<SqlitePool>) -> Json<serde_json::Value> {
    Json(json!({ "needsSetup": user_count(&pool).await == 0 }))
}

#[derive(Deserialize)]
struct SetupRequest {
    username: String,
    password: String,
}

const MIN_PASSWORD_LEN: usize = 8;

async fn setup(axum::extract::State(pool): axum::extract::State<SqlitePool>, Json(req): Json<SetupRequest>) -> Response {
    if user_count(&pool).await > 0 {
        return (StatusCode::CONFLICT, Json(json!({ "ok": false, "error": "already_set_up" }))).into_response();
    }
    let username = req.username.trim();
    if username.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "username_required" }))).into_response();
    }
    if req.password.len() < MIN_PASSWORD_LEN {
        return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "password_too_short" }))).into_response();
    }

    let hash = hash_password(&req.password);
    let user_id: i64 = sqlx::query_scalar("INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, 'Admin') RETURNING id")
        .bind(username)
        .bind(hash)
        .fetch_one(&pool)
        .await
        .expect("insert failed");

    let token = create_session(&pool, user_id).await;
    (set_cookie_header(&token), Json(json!({ "ok": true }))).into_response()
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login(axum::extract::State(pool): axum::extract::State<SqlitePool>, Json(req): Json<LoginRequest>) -> Response {
    let row = sqlx::query_as::<_, (i64, String)>("SELECT id, password_hash FROM users WHERE username = ?1")
        .bind(&req.username)
        .fetch_optional(&pool)
        .await
        .expect("query failed");

    let Some((user_id, hash)) = row else {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "ok": false }))).into_response();
    };
    if !verify_password(&req.password, &hash) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "ok": false }))).into_response();
    }

    let token = create_session(&pool, user_id).await;
    (set_cookie_header(&token), Json(json!({ "ok": true }))).into_response()
}

async fn logout(axum::extract::State(pool): axum::extract::State<SqlitePool>, headers: HeaderMap) -> Response {
    if let Some(token) = get_cookie(&headers, COOKIE_NAME) {
        sqlx::query("DELETE FROM sessions WHERE id = ?1").bind(token).execute(&pool).await.expect("delete failed");
    }
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::SET_COOKIE, format!("{COOKIE_NAME}=; Path=/; Max-Age=0").parse().unwrap());
    (response_headers, Json(json!({ "ok": true }))).into_response()
}

async fn me(axum::extract::State(pool): axum::extract::State<SqlitePool>, headers: HeaderMap) -> Json<serde_json::Value> {
    let Some(user_id) = current_user_id(&pool, &headers).await else {
        return Json(json!({ "authenticated": false }));
    };
    let google_sub: Option<String> =
        sqlx::query_scalar("SELECT google_sub FROM users WHERE id = ?1").bind(user_id).fetch_one(&pool).await.expect("query failed");
    Json(json!({ "authenticated": true, "googleLinked": google_sub.is_some() }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAccountRequest {
    current_password: String,
    new_username: Option<String>,
    new_password: Option<String>,
}

async fn update_account(editor: Editor, axum::extract::State(pool): axum::extract::State<SqlitePool>, Json(req): Json<UpdateAccountRequest>) -> Response {
    let current_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?1")
        .bind(editor.user_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    if !verify_password(&req.current_password, &current_hash) {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "ok": false, "error": "wrong_current_password" }))).into_response();
    }

    if let Some(username) = req.new_username.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query("UPDATE users SET username = ?1 WHERE id = ?2").bind(username).bind(editor.user_id).execute(&pool).await.expect("update failed");
    }
    if let Some(password) = req.new_password.as_deref().filter(|p| p.len() >= MIN_PASSWORD_LEN) {
        let hash = hash_password(password);
        sqlx::query("UPDATE users SET password_hash = ?1 WHERE id = ?2").bind(hash).bind(editor.user_id).execute(&pool).await.expect("update failed");
    }

    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}

// Called once at startup (main.rs), before the server starts accepting requests. Existing
// installs running on SPOOLBOOK_ADMIN_PASSWORD before this feature shipped get a seamless
// upgrade: no wizard, same login they already had, just backed by a real users row now instead
// of an env-var comparison. A fresh install with no env var set falls through to the wizard.
pub async fn migrate_env_var_admin_if_needed(pool: &SqlitePool) {
    if user_count(pool).await > 0 {
        return;
    }
    let Some(password) = std::env::var("SPOOLBOOK_ADMIN_PASSWORD").ok().filter(|s| !s.is_empty()) else {
        return;
    };
    let hash = hash_password(&password);
    sqlx::query("INSERT INTO users (username, password_hash, role) VALUES ('admin', ?1, 'Admin')")
        .bind(hash)
        .execute(pool)
        .await
        .expect("insert failed");
}

// Test-support only: creates a throwaway admin user + session directly against the given pool
// and returns the cookie header value other test files send to get past the Editor gate --
// mirrors the old test_only_cookie_value, but now requires a real DB row since sessions are
// stateful. Nothing in the app itself calls this.
pub async fn test_only_create_session(pool: &SqlitePool) -> String {
    let hash = hash_password("test-password-not-used-directly");
    let user_id: i64 = sqlx::query_scalar("INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, 'Admin') RETURNING id")
        .bind(format!("test-user-{}", generate_session_token()))
        .bind(hash)
        .fetch_one(pool)
        .await
        .expect("insert failed");
    let token = create_session(pool, user_id).await;
    format!("{COOKIE_NAME}={token}")
}
