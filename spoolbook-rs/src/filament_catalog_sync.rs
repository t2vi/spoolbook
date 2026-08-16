use crate::filaments::{self, FilamentInput};
use crate::settings;
use axum::http::StatusCode;
use axum::{Json, Router, extract::State, routing::post};
use sqlx::SqlitePool;

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/filaments/sync", post(sync))
}

// Mirrors FilamentCatalogParser.Parse — the published catalog's field names are PascalCase
// (verified against the live feed), same shape every additional source is expected to serve.
pub fn parse_catalog(json: &str) -> Vec<FilamentInput> {
    serde_json::from_str(json).unwrap_or_default()
}

// Mirrors FilamentService.ImportManyAsync: each entry goes through the same
// validate/dedupe/persist path as a manual create, so a bad or already-known entry is just
// skipped rather than failing the whole batch.
pub async fn import_many(pool: &SqlitePool, entries: &[FilamentInput]) -> (i64, i64) {
    let mut added = 0;
    let mut skipped = 0;
    for entry in entries {
        match filaments::create_one(pool, entry).await {
            Ok(_) => added += 1,
            Err(_) => skipped += 1,
        }
    }
    (added, skipped)
}

async fn sync(State(pool): State<SqlitePool>) -> (StatusCode, Json<serde_json::Value>) {
    let settings = settings::fetch(&pool).await;
    let additional_urls: Vec<&str> = settings
        .additional_filament_source_urls
        .as_deref()
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .collect();

    let mut entries = match reqwest::get(settings::CATALOG_URL).await {
        Ok(resp) => match resp.text().await {
            Ok(body) => parse_catalog(&body),
            Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))),
        },
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))),
    };

    // An additional source failing just means that one source is skipped this run — one broken
    // URL shouldn't block the sync of everything else (unlike the default source above).
    for url in additional_urls {
        if let Ok(resp) = reqwest::get(url).await {
            if let Ok(body) = resp.text().await {
                entries.extend(parse_catalog(&body));
            }
        }
    }

    let (added, skipped) = import_many(&pool, &entries).await;

    sqlx::query("UPDATE app_settings SET last_filament_sync_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = 1")
        .execute(&pool)
        .await
        .expect("update failed");

    (StatusCode::OK, Json(serde_json::json!({ "ok": true, "added": added, "skipped": skipped })))
}
