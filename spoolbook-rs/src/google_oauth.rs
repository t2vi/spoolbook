// Native Google sign-in, link-only for this release (docs/adr/0028): an already-authenticated
// admin links their Google account from Settings; /login's "Sign in with Google" only succeeds
// if that link already exists. No auto-provisioning a second user -- this app is single-user
// until the multi-user/print-request feature (github.com/t2vi/spoolbook/issues/91) lands.
//
// Each self-hoster registers their own Google OAuth client (Google ties a redirect URI to one
// registered app), so the client id/secret/redirect URI live in app_settings, editable from the
// Settings UI -- not env vars, so a less-technical self-hoster never has to touch docker-compose.
use crate::auth::{Editor, create_session, current_user_id, generate_session_token, set_cookie_header};
use crate::settings;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Json, Router, routing::{delete, get}};
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/auth/google/status", get(status))
        .route("/api/auth/google/config", get(get_config).put(save_config))
        .route("/api/auth/google/login", get(login))
        .route("/api/auth/google/callback", get(callback))
        .route("/api/auth/google", delete(unlink))
}

struct GoogleConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

async fn read_config_row(pool: &SqlitePool) -> (Option<String>, Option<String>, Option<String>) {
    sqlx::query_as("SELECT google_client_id, google_client_secret, google_redirect_uri FROM app_settings WHERE id = 1")
        .fetch_optional(pool)
        .await
        .expect("query failed")
        .unwrap_or((None, None, None))
}

async fn google_config(pool: &SqlitePool) -> Option<GoogleConfig> {
    let (client_id, client_secret, redirect_uri) = read_config_row(pool).await;
    match (client_id, client_secret, redirect_uri) {
        (Some(client_id), Some(client_secret), Some(redirect_uri)) if !client_id.is_empty() && !client_secret.is_empty() && !redirect_uri.is_empty() => {
            Some(GoogleConfig { client_id, client_secret, redirect_uri })
        }
        _ => None,
    }
}

async fn status(State(pool): State<SqlitePool>) -> Json<serde_json::Value> {
    Json(json!({ "configured": google_config(&pool).await.is_some() }))
}

async fn get_config(_editor: Editor, State(pool): State<SqlitePool>) -> Json<serde_json::Value> {
    let (client_id, client_secret, redirect_uri) = read_config_row(&pool).await;
    Json(json!({
        "clientId": client_id,
        "redirectUri": redirect_uri,
        "secretSet": client_secret.is_some_and(|s| !s.is_empty()),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveConfigRequest {
    client_id: String,
    client_secret: Option<String>,
    redirect_uri: String,
}

async fn save_config(_editor: Editor, State(pool): State<SqlitePool>, Json(req): Json<SaveConfigRequest>) -> Json<serde_json::Value> {
    settings::fetch(&pool).await; // ensures the id=1 row exists
    sqlx::query("UPDATE app_settings SET google_client_id = ?1, google_redirect_uri = ?2 WHERE id = 1")
        .bind(req.client_id.trim())
        .bind(req.redirect_uri.trim())
        .execute(&pool)
        .await
        .expect("update failed");
    // A blank secret on save means "keep the existing one" -- the GET endpoint never returns the
    // real secret, so there's nothing for the Settings form to re-submit unless it was retyped.
    if let Some(secret) = req.client_secret.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query("UPDATE app_settings SET google_client_secret = ?1 WHERE id = 1").bind(secret).execute(&pool).await.expect("update failed");
    }
    Json(json!({ "ok": true }))
}

fn token_endpoint() -> String {
    std::env::var("GOOGLE_TOKEN_URL").unwrap_or_else(|_| "https://oauth2.googleapis.com/token".to_string())
}

fn userinfo_endpoint() -> String {
    std::env::var("GOOGLE_USERINFO_URL").unwrap_or_else(|_| "https://www.googleapis.com/oauth2/v3/userinfo".to_string())
}

fn build_authorize_url(config: &GoogleConfig, state: &str) -> String {
    let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email")
        .append_pair("state", state)
        .append_pair("prompt", "select_account");
    url.to_string()
}

struct PendingOAuth {
    linking_user_id: Option<i64>,
    created_at: Instant,
}

const STATE_TTL: Duration = Duration::from_secs(300);

fn oauth_states() -> &'static Mutex<HashMap<String, PendingOAuth>> {
    static STATES: OnceLock<Mutex<HashMap<String, PendingOAuth>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn login(State(pool): State<SqlitePool>, headers: HeaderMap) -> Response {
    let Some(config) = google_config(&pool).await else {
        return (StatusCode::NOT_FOUND, Json(json!({ "error": "google_not_configured" }))).into_response();
    };
    let linking_user_id = current_user_id(&pool, &headers).await;
    let state = generate_session_token();
    oauth_states().lock().unwrap().insert(state.clone(), PendingOAuth { linking_user_id, created_at: Instant::now() });
    Redirect::to(&build_authorize_url(&config, &state)).into_response()
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(serde::Deserialize)]
struct GoogleUserInfo {
    sub: String,
}

// Built via reqwest::Url's own query-pair encoder rather than RequestBuilder::form (not
// available with this crate's trimmed feature set) so form-urlencoding doesn't need a new dep.
fn form_encode(pairs: &[(&str, &str)]) -> String {
    let mut url = reqwest::Url::parse("http://x/").unwrap();
    {
        let mut qp = url.query_pairs_mut();
        for (k, v) in pairs {
            qp.append_pair(k, v);
        }
    }
    url.query().unwrap_or("").to_string()
}

async fn exchange_code(config: &GoogleConfig, code: &str) -> Result<String, String> {
    let body = form_encode(&[
        ("code", code),
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
        ("redirect_uri", &config.redirect_uri),
        ("grant_type", "authorization_code"),
    ]);
    let resp = reqwest::Client::new()
        .post(token_endpoint())
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("token exchange failed: {}", resp.status()));
    }
    resp.json::<TokenResponse>().await.map(|t| t.access_token).map_err(|e| e.to_string())
}

async fn fetch_userinfo(access_token: &str) -> Result<GoogleUserInfo, String> {
    let resp = reqwest::Client::new().get(userinfo_endpoint()).bearer_auth(access_token).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("userinfo fetch failed: {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

async fn callback(Query(q): Query<CallbackQuery>, State(pool): State<SqlitePool>) -> Response {
    let (Some(code), Some(state)) = (q.code, q.state) else {
        return Redirect::to("/login?error=google_failed").into_response();
    };
    let pending = oauth_states().lock().unwrap().remove(&state);
    let Some(pending) = pending.filter(|p| p.created_at.elapsed() < STATE_TTL) else {
        return Redirect::to("/login?error=google_expired").into_response();
    };
    let Some(config) = google_config(&pool).await else {
        return Redirect::to("/login?error=google_not_configured").into_response();
    };

    let Ok(access_token) = exchange_code(&config, &code).await else {
        return Redirect::to("/login?error=google_failed").into_response();
    };
    let Ok(userinfo) = fetch_userinfo(&access_token).await else {
        return Redirect::to("/login?error=google_failed").into_response();
    };

    if let Some(user_id) = pending.linking_user_id {
        let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE google_sub = ?1").bind(&userinfo.sub).fetch_optional(&pool).await.expect("query failed");
        if existing.is_some_and(|id| id != user_id) {
            return Redirect::to("/settings?googleError=already_linked_elsewhere").into_response();
        }
        sqlx::query("UPDATE users SET google_sub = ?1 WHERE id = ?2").bind(&userinfo.sub).bind(user_id).execute(&pool).await.expect("update failed");
        return Redirect::to("/settings?googleLinked=1").into_response();
    }

    let user_id: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE google_sub = ?1").bind(&userinfo.sub).fetch_optional(&pool).await.expect("query failed");
    let Some(user_id) = user_id else {
        return Redirect::to("/login?error=google_not_linked").into_response();
    };
    let token = create_session(&pool, user_id).await;
    (set_cookie_header(&token), Redirect::to("/")).into_response()
}

// No "would this leave the account with no credential" guard: password_hash is NOT NULL today
// and every user is created via /api/setup or the env-var migration, both of which always set
// one -- so that guard can't trip yet. Tracked in github.com/t2vi/spoolbook/issues/93 to add for
// real once OAuth-only signup (issue #92) makes password_hash nullable.
async fn unlink(editor: Editor, State(pool): State<SqlitePool>) -> Response {
    sqlx::query("UPDATE users SET google_sub = NULL WHERE id = ?1").bind(editor.user_id).execute(&pool).await.expect("update failed");
    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}
