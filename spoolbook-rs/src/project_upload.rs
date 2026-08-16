use crate::projects;
use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

const MAX_BYTES: usize = 100 * 1024 * 1024;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/projects/upload", post(upload))
        .route("/api/projects/import-url", post(import_url))
}

// ponytail: fixed local path, same dev-stub tier as main.rs's hardcoded dev.db — a real
// deployment will want this configurable (persistent volume, not OS temp) once this crate has
// a deployment story at all.
fn storage_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("spoolbook-rs-projects");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn ok_response(project: projects::Project, created: bool) -> serde_json::Value {
    serde_json::json!({ "ok": true, "project": project, "created": created })
}

fn err_response(message: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": message })
}

// Content-hash-derived storage naturally dedupes re-uploads of the same bytes (upsert_by_path
// finds the existing row for that path) and means drift can never happen, since nothing but
// this function ever writes into the storage directory. Mirrors ProjectUploadService.SaveBytesAsync.
async fn save_bytes(storage_dir: &Path, pool: &SqlitePool, bytes: &[u8], display_name: &str) -> serde_json::Value {
    let hash = projects::hex(&Sha256::digest(bytes));
    let stored_path = storage_dir.join(format!("{hash}.3mf"));
    if !stored_path.exists() {
        if std::fs::write(&stored_path, bytes).is_err() {
            return err_response("Couldn't save the uploaded file.");
        }
    }

    match projects::upsert_by_path(pool, stored_path.to_str().unwrap(), display_name).await {
        Some(result) => ok_response(result.project, result.created),
        None => err_response("file_not_found"),
    }
}

async fn upload(State(pool): State<SqlitePool>, mut multipart: Multipart) -> (StatusCode, Json<serde_json::Value>) {
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(field)) if field.name() == Some("file") => break Some(field),
            Ok(Some(_)) => continue,
            Ok(None) => break None,
            Err(_) => return (StatusCode::BAD_REQUEST, Json(err_response("Expected multipart form data."))),
        }
    };
    let Some(field) = field else {
        return (StatusCode::BAD_REQUEST, Json(err_response("No file provided.")));
    };

    let file_name = field.file_name().unwrap_or("upload.3mf").to_string();
    let bytes = match field.bytes().await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(err_response("Couldn't read the uploaded file."))),
    };

    let body = save_bytes(&storage_dir(), &pool, &bytes, &file_name).await;
    let status = if body["ok"] == true { StatusCode::OK } else { StatusCode::BAD_REQUEST };
    (status, Json(body))
}

#[derive(Deserialize)]
struct ImportUrlRequest {
    url: String,
}

// Generic URL fetch (docs/adr/0023) — any direct link to a .3mf file, including a MakerWorld
// download link copied manually. Auto-resolving a MakerWorld page URL itself is deferred: that
// needs their unofficial frontend API, not a direct file link.
async fn import_url(State(pool): State<SqlitePool>, Json(req): Json<ImportUrlRequest>) -> (StatusCode, Json<serde_json::Value>) {
    let Ok(parsed) = reqwest::Url::parse(&req.url) else {
        return (StatusCode::BAD_REQUEST, Json(err_response("Enter a valid http(s) URL.")));
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return (StatusCode::BAD_REQUEST, Json(err_response("Enter a valid http(s) URL.")));
    }

    let response = match reqwest::get(parsed.clone()).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(err_response(&format!("Fetch failed: {e}")))),
    };
    if !response.status().is_success() {
        return (StatusCode::BAD_REQUEST, Json(err_response(&format!("Fetch failed: HTTP {}", response.status().as_u16()))));
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(err_response(&format!("Fetch failed: {e}")))),
    };
    if bytes.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(err_response("Downloaded file was empty.")));
    }
    if bytes.len() > MAX_BYTES {
        return (StatusCode::BAD_REQUEST, Json(err_response("Downloaded file is too large (over 100 MB).")));
    }

    let file_name = parsed
        .path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|name| name.to_lowercase().ends_with(".3mf"))
        .unwrap_or("download.3mf")
        .to_string();

    let body = save_bytes(&storage_dir(), &pool, &bytes, &file_name).await;
    let status = if body["ok"] == true { StatusCode::OK } else { StatusCode::BAD_REQUEST };
    (status, Json(body))
}
