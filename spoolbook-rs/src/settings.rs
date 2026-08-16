use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub const CATALOG_URL: &str =
    "https://raw.githubusercontent.com/t2vi/spoolbook-filament-sync/main/data/filament-catalog.json";

#[derive(sqlx::FromRow)]
pub struct AppSettings {
    pub last_filament_sync_at: Option<String>,
    pub additional_filament_source_urls: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsResponse {
    additional_filament_source_urls: Option<String>,
    last_filament_sync_at: Option<String>,
    catalog_url: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsRequest {
    additional_filament_source_urls: Option<String>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/settings", get(get_settings).post(save_settings))
}

// Lazily creates the single settings row (id = 1) on first read, matching AppSettingsService.
pub async fn fetch(pool: &SqlitePool) -> AppSettings {
    if let Some(row) = sqlx::query_as::<_, AppSettings>(
        "SELECT last_filament_sync_at, additional_filament_source_urls FROM app_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .expect("query failed")
    {
        return row;
    }

    sqlx::query("INSERT INTO app_settings (id) VALUES (1)").execute(pool).await.expect("insert failed");
    AppSettings { last_filament_sync_at: None, additional_filament_source_urls: None }
}

async fn get_settings(State(pool): State<SqlitePool>) -> Json<SettingsResponse> {
    let settings = fetch(&pool).await;
    Json(SettingsResponse {
        additional_filament_source_urls: settings.additional_filament_source_urls,
        last_filament_sync_at: settings.last_filament_sync_at,
        catalog_url: CATALOG_URL,
    })
}

async fn save_settings(State(pool): State<SqlitePool>, Json(req): Json<SaveSettingsRequest>) -> Json<serde_json::Value> {
    fetch(&pool).await;
    let urls = req.additional_filament_source_urls.filter(|u| !u.trim().is_empty());
    sqlx::query("UPDATE app_settings SET additional_filament_source_urls = ?1 WHERE id = 1")
        .bind(urls)
        .execute(&pool)
        .await
        .expect("update failed");

    Json(serde_json::json!({ "ok": true }))
}
