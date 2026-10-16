use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FilamentColor {
    pub id: i64,
    pub name: String,
    pub hex: String,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/filament-colors", get(list).post(create))
        .route("/api/filament-colors/inventory", get(inventory))
        .route("/api/filament-colors/{id}", axum::routing::put(update).delete(delete))
}

async fn list(State(pool): State<SqlitePool>) -> Json<Vec<FilamentColor>> {
    let colors = sqlx::query_as::<_, FilamentColor>("SELECT id, name, hex FROM filament_colors ORDER BY name")
        .fetch_all(&pool)
        .await
        .expect("query failed");

    Json(colors)
}

// One #RRGGBB, or a comma-separated list of them for a multi-color filament (black + gold =
// "#1A1A1A, #D4AF37"). Returns the normalized form: parts trimmed, empty ones dropped.
pub fn normalize_hex(hex: &str) -> Option<String> {
    let parts: Vec<&str> = hex.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
    let valid = |p: &&str| p.len() == 7 && p.starts_with('#') && p[1..].chars().all(|c| c.is_ascii_hexdigit());
    (!parts.is_empty() && parts.iter().all(valid)).then(|| parts.join(", "))
}

#[derive(Deserialize)]
struct ColorInput {
    name: String,
    hex: String,
}

type ColorResponse = (StatusCode, Json<serde_json::Value>);

fn bad_request(error: &str) -> ColorResponse {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": error })))
}

fn ok(color: FilamentColor) -> ColorResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true, "color": color })))
}

// Shared by create and update: Ok((name, hex)) or the 400 to return. `own_id` excludes the color
// being edited from the duplicate check.
async fn validate(pool: &SqlitePool, input: &ColorInput, own_id: Option<i64>) -> Result<(String, String), ColorResponse> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(bad_request("Name is required"));
    }
    let Some(hex) = normalize_hex(&input.hex) else { return Err(bad_request("invalid_hex")) };
    let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM filament_colors WHERE name = ?1 AND id IS NOT ?2)")
        .bind(name)
        .bind(own_id)
        .fetch_one(pool)
        .await
        .expect("query failed");
    if duplicate {
        return Err(bad_request("duplicate"));
    }
    Ok((name.to_string(), hex))
}

async fn create(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Json(input): Json<ColorInput>) -> ColorResponse {
    let (name, hex) = match validate(&pool, &input, None).await {
        Ok(v) => v,
        Err(response) => return response,
    };
    let color = sqlx::query_as::<_, FilamentColor>("INSERT INTO filament_colors (name, hex) VALUES (?1, ?2) RETURNING id, name, hex")
        .bind(name)
        .bind(hex)
        .fetch_one(&pool)
        .await
        .expect("insert failed");
    ok(color)
}

async fn update(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Path(id): Path<i64>, Json(input): Json<ColorInput>) -> ColorResponse {
    let Some(old_name) = sqlx::query_scalar::<_, String>("SELECT name FROM filament_colors WHERE id = ?1").bind(id).fetch_optional(&pool).await.expect("query failed")
    else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "not_found" })));
    };
    let (name, hex) = match validate(&pool, &input, Some(id)).await {
        Ok(v) => v,
        Err(response) => return response,
    };

    // Filaments refer to a color by name, so a rename carries them along or they'd lose their swatch.
    let mut tx = pool.begin().await.expect("begin failed");
    sqlx::query("UPDATE filaments SET color = ?1 WHERE color = ?2").bind(&name).bind(&old_name).execute(&mut *tx).await.expect("update failed");
    let color = sqlx::query_as::<_, FilamentColor>("UPDATE filament_colors SET name = ?1, hex = ?2 WHERE id = ?3 RETURNING id, name, hex")
        .bind(name)
        .bind(hex)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .expect("update failed");
    tx.commit().await.expect("commit failed");
    ok(color)
}

async fn delete(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Path(id): Path<i64>) -> ColorResponse {
    let in_use: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM filaments f WHERE f.color = c.name) FROM filament_colors c WHERE c.id = ?1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("query failed");
    match in_use {
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "not_found" }))),
        Some(true) => bad_request("in_use"),
        Some(false) => {
            sqlx::query("DELETE FROM filament_colors WHERE id = ?1").bind(id).execute(&pool).await.expect("delete failed");
            (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryQuery {
    q: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn inventory(State(pool): State<SqlitePool>, Query(q): Query<InventoryQuery>) -> Json<serde_json::Value> {
    let page = q.page.filter(|p| *p > 0).unwrap_or(1);
    let page_size = q.page_size.filter(|p| *p > 0).unwrap_or(20);
    // Whitelisted, never interpolated from the request as-is.
    let column = if q.sort.as_deref() == Some("hex") { "hex" } else { "name" };
    let direction = if q.order.as_deref() == Some("desc") { "DESC" } else { "ASC" };
    let pattern = format!("%{}%", q.q.as_deref().unwrap_or("").trim());

    // LIKE is case-insensitive for ASCII in SQLite.
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM filament_colors WHERE name LIKE ?1").bind(&pattern).fetch_one(&pool).await.expect("query failed");
    let colors = sqlx::query_as::<_, FilamentColor>(&format!(
        "SELECT id, name, hex FROM filament_colors WHERE name LIKE ?1 ORDER BY {column} {direction}, name LIMIT ?2 OFFSET ?3"
    ))
    .bind(&pattern)
    .bind(page_size)
    .bind((page - 1) * page_size)
    .fetch_all(&pool)
    .await
    .expect("query failed");
    let total_pages = ((total + page_size - 1) / page_size).max(1);
    Json(serde_json::json!({ "colors": colors, "total": total, "page": page, "pageSize": page_size, "totalPages": total_pages }))
}
