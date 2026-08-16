use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Filament {
    pub id: i64,
    pub brand: String,
    pub material: String,
    pub variant: Option<String>,
    pub color: String,
}

// Aliases accept the catalog feed's PascalCase field names (Brand/Material/Variant/Color/Hex)
// alongside the API's lowercase ones, so the same type deserializes both without a second
// struct — matches .NET's PropertyNameCaseInsensitive on the catalog parser.
#[derive(Deserialize)]
pub struct FilamentInput {
    #[serde(alias = "Brand")]
    pub brand: String,
    #[serde(alias = "Material")]
    pub material: String,
    #[serde(alias = "Variant")]
    pub variant: Option<String>,
    #[serde(alias = "Color")]
    pub color: String,
    // Set by the catalog scraper when the color name matched a known hex; falls back to the
    // #CCCCCC placeholder in ensure_color_exists when absent.
    #[serde(alias = "Hex")]
    pub hex: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilamentResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Filament>,
}

#[derive(Deserialize)]
pub struct FilamentQuery {
    brand: Option<String>,
    material: Option<String>,
    page: Option<i64>,
    #[serde(rename = "pageSize")]
    page_size: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilamentSearchResult {
    entries: Vec<Filament>,
    total: i64,
    page: i64,
    page_size: i64,
    total_pages: i64,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/filaments", get(search).post(create))
        .route("/api/filaments/all", get(list_all))
        .route("/api/filaments/{id}", axum::routing::put(update).delete(delete))
}

async fn search(State(pool): State<SqlitePool>, Query(q): Query<FilamentQuery>) -> Json<FilamentSearchResult> {
    let page = q.page.filter(|&p| p > 0).unwrap_or(1);
    let page_size = q.page_size.filter(|&s| s > 0).unwrap_or(20);
    let brand = q.brand.filter(|b| !b.is_empty());
    let material = q.material.filter(|m| !m.is_empty());

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM filaments
         WHERE (?1 IS NULL OR brand = ?1) AND (?2 IS NULL OR material = ?2)",
    )
    .bind(&brand)
    .bind(&material)
    .fetch_one(&pool)
    .await
    .expect("query failed");

    let entries = sqlx::query_as::<_, Filament>(
        "SELECT id, brand, material, variant, color FROM filaments
         WHERE (?1 IS NULL OR brand = ?1) AND (?2 IS NULL OR material = ?2)
         ORDER BY brand, material
         LIMIT ?3 OFFSET ?4",
    )
    .bind(&brand)
    .bind(&material)
    .bind(page_size)
    .bind((page - 1) * page_size)
    .fetch_all(&pool)
    .await
    .expect("query failed");

    let total_pages = ((total as f64) / (page_size as f64)).ceil().max(1.0) as i64;

    Json(FilamentSearchResult { entries, total, page, page_size, total_pages })
}

async fn list_all(State(pool): State<SqlitePool>) -> Json<Vec<Filament>> {
    let filaments = sqlx::query_as::<_, Filament>(
        "SELECT id, brand, material, variant, color FROM filaments ORDER BY brand, material",
    )
    .fetch_all(&pool)
    .await
    .expect("query failed");

    Json(filaments)
}

fn validate(input: &FilamentInput) -> Option<&'static str> {
    if input.brand.trim().is_empty() {
        return Some("Brand is required");
    }
    if input.material.trim().is_empty() {
        return Some("Material is required");
    }
    if input.color.trim().is_empty() {
        return Some("Color is required");
    }
    None
}

// exclude_id lets update check for duplicates against every *other* row without tripping on
// the row being edited saving its own unchanged values back.
async fn is_duplicate(pool: &SqlitePool, input: &FilamentInput, exclude_id: Option<i64>) -> bool {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM filaments
         WHERE brand = ?1 AND material = ?2 AND variant IS ?3 AND color = ?4
         AND (?5 IS NULL OR id != ?5)",
    )
    .bind(&input.brand)
    .bind(&input.material)
    .bind(&input.variant)
    .bind(&input.color)
    .bind(exclude_id)
    .fetch_one(pool)
    .await
    .expect("query failed");

    count > 0
}

fn err(status: StatusCode, message: &str) -> (StatusCode, Json<FilamentResult>) {
    (status, Json(FilamentResult { ok: false, error: Some(message.to_string()), entry: None }))
}

// Colors aren't hand-seeded independently — they're discovered from filaments (known or owned)
// as they're added. Uses the catalog scraper's resolved hex when available, otherwise a
// placeholder the user can correct in Settings.
async fn ensure_color_exists(pool: &SqlitePool, name: &str, hex: Option<&str>) {
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM filament_colors WHERE name = ?1")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("query failed")
        > 0;
    if exists {
        return;
    }

    sqlx::query("INSERT INTO filament_colors (name, hex) VALUES (?1, ?2)")
        .bind(name)
        .bind(hex.unwrap_or("#CCCCCC"))
        .execute(pool)
        .await
        .expect("insert failed");
}

// Shared by the create endpoint and the catalog sync importer, which needs the same
// validate/dedupe/persist path per entry without the axum request/response plumbing.
pub async fn create_one(pool: &SqlitePool, input: &FilamentInput) -> Result<Filament, &'static str> {
    if let Some(error) = validate(input) {
        return Err(error);
    }
    if is_duplicate(pool, input, None).await {
        return Err("duplicate");
    }

    let entry = sqlx::query_as::<_, Filament>(
        "INSERT INTO filaments (brand, material, variant, color) VALUES (?1, ?2, ?3, ?4)
         RETURNING id, brand, material, variant, color",
    )
    .bind(&input.brand)
    .bind(&input.material)
    .bind(&input.variant)
    .bind(&input.color)
    .fetch_one(pool)
    .await
    .expect("insert failed");

    ensure_color_exists(pool, &input.color, input.hex.as_deref()).await;

    Ok(entry)
}

async fn create(State(pool): State<SqlitePool>, Json(input): Json<FilamentInput>) -> (StatusCode, Json<FilamentResult>) {
    match create_one(&pool, &input).await {
        Ok(entry) => (StatusCode::OK, Json(FilamentResult { ok: true, error: None, entry: Some(entry) })),
        Err(error) => err(StatusCode::BAD_REQUEST, error),
    }
}

async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<FilamentInput>,
) -> (StatusCode, Json<FilamentResult>) {
    if let Some(error) = validate(&input) {
        return err(StatusCode::BAD_REQUEST, error);
    }
    if is_duplicate(&pool, &input, Some(id)).await {
        return err(StatusCode::BAD_REQUEST, "duplicate");
    }

    let entry = sqlx::query_as::<_, Filament>(
        "UPDATE filaments SET brand = ?1, material = ?2, variant = ?3, color = ?4
         WHERE id = ?5
         RETURNING id, brand, material, variant, color",
    )
    .bind(&input.brand)
    .bind(&input.material)
    .bind(&input.variant)
    .bind(&input.color)
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("update failed");

    match entry {
        Some(entry) => {
            ensure_color_exists(&pool, &input.color, input.hex.as_deref()).await;
            (StatusCode::OK, Json(FilamentResult { ok: true, error: None, entry: Some(entry) }))
        }
        None => err(StatusCode::NOT_FOUND, "not_found"),
    }
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<FilamentResult>) {
    let has_spools = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM spools WHERE filament_id = ?1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("query failed")
        > 0;

    if has_spools {
        return err(StatusCode::BAD_REQUEST, "has_spools");
    }

    let result = sqlx::query("DELETE FROM filaments WHERE id = ?1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete failed");

    if result.rows_affected() == 0 {
        return err(StatusCode::NOT_FOUND, "not_found");
    }

    (StatusCode::OK, Json(FilamentResult { ok: true, error: None, entry: None }))
}
