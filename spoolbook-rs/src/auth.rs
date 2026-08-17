// Port of the shared-secret editor gate (Program.cs's cookie auth block + TrySignInEditorAsync).
// Single editor, no user table, no OAuth — same threat model as the C# original (docs/adr/0005's
// v2 mutation lock, reactivated for the LAN pivot). Only the SvelteKit-facing JSON endpoints
// (/api/login, /api/logout, /api/me) are ported — the HTML form-post /login /logout routes exist
// in C# only for the retired Blazor era and have no caller left once the frontend is SvelteKit.
//
// Cookie value is HMAC-SHA256(SPOOLBOOK_ADMIN_PASSWORD, "editor") rather than a signed/encrypted
// session token: the "identity" here is a single constant ("editor"), so there's nothing to
// encode — the cookie IS the proof of knowing the password, deterministic across logins, checked
// by recomputing and comparing. No expiry, matching the C# cookie's own default lifetime
// (persists until logout).
use axum::extract::FromRequestParts;
use axum::http::{HeaderMap, StatusCode, header, request::Parts};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::get};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

const COOKIE_NAME: &str = "spoolbook_editor";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// Hand-rolled rather than pulling in the `hmac` crate: RustCrypto's published `hmac` (0.12) is
// pinned to `digest` 0.10, but this crate already depends on `sha2` 0.11 (digest 0.11) elsewhere
// (projects.rs's mesh-hash) — a second, incompatible digest major version isn't worth it for one
// call site. HMAC itself is just this fixed composition around any hash (RFC 2104), not a
// primitive being reinvented.
const HMAC_BLOCK_SIZE: usize = 64; // SHA-256's block size

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut key_block = [0u8; HMAC_BLOCK_SIZE];
    if key.len() > HMAC_BLOCK_SIZE {
        let hashed = Sha256::digest(key);
        key_block[..hashed.len()].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; HMAC_BLOCK_SIZE];
    let mut opad = [0x5cu8; HMAC_BLOCK_SIZE];
    for i in 0..HMAC_BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().to_vec()
}

fn expected_token() -> Option<String> {
    let secret = std::env::var("SPOOLBOOK_ADMIN_PASSWORD").ok().filter(|s| !s.is_empty())?;
    Some(hex(&hmac_sha256(secret.as_bytes(), b"editor")))
}

fn get_cookie<'h>(headers: &'h HeaderMap, name: &str) -> Option<&'h str> {
    headers.get(header::COOKIE)?.to_str().ok()?.split(';').map(str::trim).find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == name).then_some(v)
    })
}

fn is_authenticated(headers: &HeaderMap) -> bool {
    match (expected_token(), get_cookie(headers, COOKIE_NAME)) {
        (Some(expected), Some(actual)) => expected == actual,
        _ => false,
    }
}

// Extractor form of the auth gate — add `_editor: Editor` as a handler parameter to require it,
// same effect as C#'s per-route `.RequireAuthorization()`. Handlers that don't take it stay
// public, matching every GET (read) route in the original.
pub struct Editor;

pub struct AuthError;

impl IntoResponse for AuthError {
    // Matches Program.cs's OnRedirectToLogin API branch: a clean 401 with {authenticated:false},
    // not an HTML redirect — every route in this crate is a JSON API.
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, Json(json!({ "authenticated": false }))).into_response()
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Editor {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if is_authenticated(&parts.headers) { Ok(Editor) } else { Err(AuthError) }
    }
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/login", axum::routing::post(login)).route("/api/logout", axum::routing::post(logout)).route("/api/me", get(me))
}

#[derive(Deserialize)]
struct LoginRequest {
    password: String,
}

fn set_cookie_header(value: String) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, format!("{COOKIE_NAME}={value}; Path=/; HttpOnly; SameSite=Lax").parse().unwrap());
    headers
}

async fn login(Json(req): Json<LoginRequest>) -> Response {
    let Some(expected_password) = std::env::var("SPOOLBOOK_ADMIN_PASSWORD").ok().filter(|s| !s.is_empty()) else {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "ok": false }))).into_response();
    };
    if req.password != expected_password {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "ok": false }))).into_response();
    }

    let token = expected_token().expect("password just verified non-empty above");
    (set_cookie_header(token), Json(json!({ "ok": true }))).into_response()
}

async fn logout() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, format!("{COOKIE_NAME}=; Path=/; Max-Age=0").parse().unwrap());
    (headers, Json(json!({ "ok": true }))).into_response()
}

async fn me(headers: HeaderMap) -> Json<serde_json::Value> {
    Json(json!({ "authenticated": is_authenticated(&headers) }))
}

// Test-support only: computes the cookie value a real /api/login with this password would
// produce, so integration tests can skip the login round-trip. Nothing in the app itself calls
// this.
pub fn test_only_cookie_value(password: &str) -> String {
    hex(&hmac_sha256(password.as_bytes(), b"editor"))
}

#[cfg(test)]
mod tests {
    use super::hmac_sha256;

    // RFC 4231 test case 1 — the one runnable check on the hand-rolled HMAC construction itself,
    // independent of the login/cookie plumbing exercised in tests/auth.rs.
    #[test]
    fn matches_rfc_4231_test_case_1() {
        let key = [0x0bu8; 20];
        let data = b"Hi There";
        let expected = "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7";

        let result = hmac_sha256(&key, data);
        let hex: String = result.iter().map(|b| format!("{b:02x}")).collect();

        assert_eq!(hex, expected);
    }
}
