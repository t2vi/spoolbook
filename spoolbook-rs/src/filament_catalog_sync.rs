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
    try_parse_catalog(json).unwrap_or_default()
}

// Strict form for the published catalog: a body that isn't a JSON catalog is a failed sync, not
// an empty one (an empty list is still fine).
pub fn try_parse_catalog(json: &str) -> Result<Vec<FilamentInput>, String> {
    serde_json::from_str(json).map_err(|e| format!("filament catalog isn't valid JSON: {e}"))
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client.get(url).send().await.map_err(|e| format!("couldn't fetch {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("couldn't fetch {url}: HTTP {}", resp.status().as_u16()));
    }
    resp.text().await.map_err(|e| format!("couldn't read {url}: {e}"))
}

// The published feed has scraper artifacts: every Bambu Lab row arrives as material `<span`,
// variant `class="align-middle">PLA Matte</span>` (a title with its HTML, split at the first
// space). Join, drop the tags, re-split at the first space (007/FR-025). None = already clean.
pub fn clean_markup(material: &str, variant: Option<&str>) -> Option<(String, Option<String>)> {
    let joined = format!("{material} {}", variant.unwrap_or(""));
    if !joined.contains(['<', '>']) {
        return None;
    }
    let mut text = String::new();
    let mut in_tag = false;
    for c in joined.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let (first, rest) = words.split_first()?;
    Some((first.to_string(), (!rest.is_empty()).then(|| rest.join(" "))))
}

// Startup repair of filaments already imported with markup (007/FR-026): rename in place, or,
// when the cleaned name already exists, move its spools and profiles there and drop it.
pub async fn repair_markup(pool: &SqlitePool) {
    let broken: Vec<(i64, String, String, Option<String>, String)> =
        sqlx::query_as("SELECT id, brand, material, variant, color FROM filaments WHERE material LIKE '%<%' OR material LIKE '%>%' OR variant LIKE '%<%' OR variant LIKE '%>%'")
            .fetch_all(pool)
            .await
            .expect("query failed");
    for (id, brand, material, variant, color) in broken {
        let Some((material, variant)) = clean_markup(&material, variant.as_deref()) else { continue };
        let mut tx = pool.begin().await.expect("begin failed");
        let twin: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM filaments WHERE brand = ?1 AND material = ?2 AND variant IS ?3 AND color = ?4 AND id != ?5",
        )
        .bind(&brand)
        .bind(&material)
        .bind(&variant)
        .bind(&color)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .expect("query failed");
        match twin {
            Some(twin) => {
                for table in ["spools", "print_profiles"] {
                    sqlx::query(&format!("UPDATE {table} SET filament_id = ?1 WHERE filament_id = ?2")).bind(twin).bind(id).execute(&mut *tx).await.expect("update failed");
                }
                sqlx::query("DELETE FROM filaments WHERE id = ?1").bind(id).execute(&mut *tx).await.expect("delete failed");
            }
            None => {
                sqlx::query("UPDATE filaments SET material = ?1, variant = ?2 WHERE id = ?3")
                    .bind(&material)
                    .bind(&variant)
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .expect("update failed");
            }
        }
        tx.commit().await.expect("commit failed");
    }
}

// Mirrors FilamentService.ImportManyAsync: each entry goes through the same
// validate/dedupe/persist path as a manual create, so a bad or already-known entry is just
// skipped rather than failing the whole batch.
pub async fn import_many(pool: &SqlitePool, entries: &[FilamentInput]) -> (i64, i64) {
    let mut added = 0;
    let mut skipped = 0;
    for entry in entries {
        let cleaned;
        let entry = match clean_markup(&entry.material, entry.variant.as_deref()) {
            Some((material, variant)) => {
                cleaned = FilamentInput { material, variant, ..entry.clone() };
                &cleaned
            }
            None => entry,
        };
        match filaments::create_one(pool, entry).await {
            Ok(_) => added += 1,
            Err(_) => skipped += 1,
        }
        refresh_color_hex(pool, entry).await;
    }
    (added, skipped)
}

// The feed's hex wins over what an earlier sync stored — e.g. a multi-color filament's list
// replacing a single guess — but never over a hex a person set (007/FR-027).
async fn refresh_color_hex(pool: &SqlitePool, entry: &FilamentInput) {
    let Some(hex) = entry.hex.as_deref().and_then(crate::colors::normalize_hex) else { return };
    sqlx::query("UPDATE filament_colors SET hex = ?1 WHERE name = ?2 AND hex_set_by_user = 0 AND hex <> ?1")
        .bind(hex)
        .bind(entry.color.trim())
        .execute(pool)
        .await
        .expect("update failed");
}

// True when the catalog has never been synced, or the last sync was over 24h ago — shared by
// the manual /api/filaments/sync button and the startup auto-sync in main.rs, same throttle
// AppSettings.LastFilamentSyncAt gave the .NET app.
pub fn should_sync(last_filament_sync_at: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> bool {
    match last_filament_sync_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()) {
        Some(last) => now - last.with_timezone(&chrono::Utc) > chrono::Duration::hours(24),
        None => true,
    }
}

// Shared by the manual /api/filaments/sync button and main.rs's startup auto-sync.
pub async fn run_sync(pool: &SqlitePool) -> Result<(i64, i64), String> {
    let settings = settings::fetch(pool).await;
    let additional_urls: Vec<&str> = settings
        .additional_filament_source_urls
        .as_deref()
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .collect();

    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build().map_err(|e| e.to_string())?;
    // Any failure here returns before last_filament_sync_at is stamped, so the startup auto-sync
    // retries next launch instead of waiting out the 24h throttle on a sync that never happened.
    let mut entries = try_parse_catalog(&fetch_text(&client, &settings::catalog_url()).await?)?;

    // An additional source failing just means that one source is skipped this run — one broken
    // URL shouldn't block the sync of everything else (unlike the default source above).
    for url in additional_urls {
        if let Ok(body) = fetch_text(&client, url).await {
            entries.extend(parse_catalog(&body));
        }
    }

    let (added, skipped) = import_many(pool, &entries).await;

    sqlx::query("UPDATE app_settings SET last_filament_sync_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = 1")
        .execute(pool)
        .await
        .expect("update failed");

    Ok((added, skipped))
}

async fn sync(_editor: crate::auth::Editor, State(pool): State<SqlitePool>) -> (StatusCode, Json<serde_json::Value>) {
    match run_sync(&pool).await {
        Ok((added, skipped)) => (StatusCode::OK, Json(serde_json::json!({ "ok": true, "added": added, "skipped": skipped }))),
        Err(error) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": error }))),
    }
}
