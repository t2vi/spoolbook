use crate::filaments::Filament;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(sqlx::FromRow)]
struct SpoolRow {
    id: i64,
    filament_id: i64,
    lot_code: Option<String>,
    purchased_at: Option<String>,
    opened_at: Option<String>,
    emptied_at: Option<String>,
    weight_grams: Option<i64>,
    diameter_mm: Option<f64>,
    notes: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spool {
    pub id: i64,
    pub filament_id: i64,
    pub filament: Option<Filament>,
    pub lot_code: Option<String>,
    pub purchased_at: Option<String>,
    pub opened_at: Option<String>,
    pub emptied_at: Option<String>,
    pub weight_grams: Option<i64>,
    pub diameter_mm: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpoolInput {
    pub lot_code: Option<String>,
    pub purchased_at: Option<String>,
    pub opened_at: Option<String>,
    pub emptied_at: Option<String>,
    pub weight_grams: Option<i64>,
    pub diameter_mm: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSpoolInput {
    pub filament_id: i64,
    #[serde(flatten)]
    pub input: SpoolInput,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpoolResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spool: Option<Spool>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/spools", get(list_all).post(create))
        .route("/api/spools/inventory", get(inventory))
        .route("/api/spools/{id}", get(get_one).put(update).delete(delete))
}

// One extra query per spool to fetch its filament — fine at this app's scale (single user,
// a handful of spools), simplest thing that works. Revisit with a JOIN if the list ever grows
// large enough for N+1 to matter.
async fn attach_filament(pool: &SqlitePool, row: SpoolRow) -> Spool {
    let filament = sqlx::query_as::<_, Filament>(
        "SELECT id, brand, material, variant, color FROM filaments WHERE id = ?1",
    )
    .bind(row.filament_id)
    .fetch_optional(pool)
    .await
    .expect("query failed");

    Spool {
        id: row.id,
        filament_id: row.filament_id,
        filament,
        lot_code: row.lot_code,
        purchased_at: row.purchased_at,
        opened_at: row.opened_at,
        emptied_at: row.emptied_at,
        weight_grams: row.weight_grams,
        diameter_mm: row.diameter_mm,
        notes: row.notes,
    }
}

async fn list_all(State(pool): State<SqlitePool>) -> Json<Vec<Spool>> {
    let rows = sqlx::query_as::<_, SpoolRow>(
        "SELECT id, filament_id, lot_code, purchased_at, opened_at, emptied_at, weight_grams, diameter_mm, notes
         FROM spools ORDER BY created_at",
    )
    .fetch_all(&pool)
    .await
    .expect("query failed");

    let mut spools = Vec::with_capacity(rows.len());
    for row in rows {
        spools.push(attach_filament(&pool, row).await);
    }

    Json(spools)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryQuery {
    brand: Option<String>,
    material: Option<String>,
    status: Option<String>,
    purchased_from: Option<String>,
    purchased_to: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

// The spool list's filtered, sorted, paged view (007/FR-034).
async fn inventory(State(pool): State<SqlitePool>, Query(q): Query<InventoryQuery>) -> Json<serde_json::Value> {
    let page = q.page.filter(|p| *p > 0).unwrap_or(1);
    let page_size = q.page_size.filter(|p| *p > 0).unwrap_or(20);
    let direction = if q.order.as_deref() == Some("desc") { "DESC" } else { "ASC" };
    // Whitelisted, never interpolated from the request as-is. Date sorts put spools without that
    // date last in either direction.
    let order_by = match q.sort.as_deref() {
        Some(col @ ("purchasedAt" | "openedAt" | "emptiedAt")) => {
            let col = match col { "purchasedAt" => "purchased_at", "openedAt" => "opened_at", _ => "emptied_at" };
            format!("s.{col} IS NULL, s.{col} {direction}, s.created_at")
        }
        _ => format!("f.brand {direction}, f.material {direction}, s.created_at"),
    };
    let status = match q.status.as_deref() {
        Some("Unopened") => "s.opened_at IS NULL",
        Some("Opened") => "s.opened_at IS NOT NULL AND s.emptied_at IS NULL",
        Some("Empty") => "s.emptied_at IS NOT NULL",
        _ => "1",
    };
    let filter = format!(
        "FROM spools s JOIN filaments f ON f.id = s.filament_id
         WHERE (?1 IS NULL OR f.brand = ?1) AND (?2 IS NULL OR f.material = ?2) AND {status}
           AND (?3 IS NULL OR substr(s.purchased_at, 1, 10) >= ?3) AND (?4 IS NULL OR substr(s.purchased_at, 1, 10) <= ?4)"
    );
    let blank_is_none = |v: Option<String>| v.filter(|v| !v.trim().is_empty());
    let (brand, material, from, to) = (blank_is_none(q.brand), blank_is_none(q.material), blank_is_none(q.purchased_from), blank_is_none(q.purchased_to));

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) {filter}"))
        .bind(&brand)
        .bind(&material)
        .bind(&from)
        .bind(&to)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    let rows = sqlx::query_as::<_, SpoolRow>(&format!(
        "SELECT s.id, s.filament_id, s.lot_code, s.purchased_at, s.opened_at, s.emptied_at, s.weight_grams, s.diameter_mm, s.notes
         {filter} ORDER BY {order_by} LIMIT ?5 OFFSET ?6"
    ))
    .bind(&brand)
    .bind(&material)
    .bind(&from)
    .bind(&to)
    .bind(page_size)
    .bind((page - 1) * page_size)
    .fetch_all(&pool)
    .await
    .expect("query failed");

    let mut spools = Vec::with_capacity(rows.len());
    for row in rows {
        spools.push(attach_filament(&pool, row).await);
    }
    let total_pages = ((total + page_size - 1) / page_size).max(1);
    Json(serde_json::json!({ "spools": spools, "total": total, "page": page, "pageSize": page_size, "totalPages": total_pages }))
}

async fn get_one(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Result<Json<Spool>, StatusCode> {
    match fetch_by_id(&pool, id).await {
        Some(spool) => Ok(Json(spool)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// Exposed for Print's nested `spool` field (Print references a Spool directly, same as .NET's
// `.Include(p => p.Spool).ThenInclude(s => s!.Filament)`).
pub async fn fetch_by_id(pool: &SqlitePool, id: i64) -> Option<Spool> {
    let row = sqlx::query_as::<_, SpoolRow>(
        "SELECT id, filament_id, lot_code, purchased_at, opened_at, emptied_at, weight_grams, diameter_mm, notes
         FROM spools WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .expect("query failed")?;

    Some(attach_filament(pool, row).await)
}

const DATES_OUT_OF_ORDER: &str = "Dates are out of order — purchased, then opened, then emptied.";

// Compared by calendar day (first 10 chars of the ISO date/datetime), so a date-only value and a
// full timestamp on the same day still count as in order.
fn dates_in_order(input: &SpoolInput) -> bool {
    let day = |d: &Option<String>| d.as_deref().map(|d| d.get(..10).unwrap_or(d).to_string());
    let dates: Vec<String> = [&input.purchased_at, &input.opened_at, &input.emptied_at].into_iter().filter_map(day).collect();
    dates.windows(2).all(|w| w[0] <= w[1])
}

fn err(status: StatusCode, message: &str) -> (StatusCode, Json<SpoolResult>) {
    (status, Json(SpoolResult { ok: false, error: Some(message.to_string()), spool: None }))
}

async fn create(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Json(body): Json<CreateSpoolInput>,
) -> (StatusCode, Json<SpoolResult>) {
    let input = body.input;
    let filament_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM filaments WHERE id = ?1)")
        .bind(body.filament_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    if !filament_exists {
        return err(StatusCode::BAD_REQUEST, "Unknown filament.");
    }
    if !dates_in_order(&input) {
        return err(StatusCode::BAD_REQUEST, DATES_OUT_OF_ORDER);
    }
    let row = sqlx::query_as::<_, SpoolRow>(
        "INSERT INTO spools (filament_id, lot_code, purchased_at, opened_at, emptied_at, weight_grams, diameter_mm, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         RETURNING id, filament_id, lot_code, purchased_at, opened_at, emptied_at, weight_grams, diameter_mm, notes",
    )
    .bind(body.filament_id)
    .bind(&input.lot_code)
    .bind(&input.purchased_at)
    .bind(&input.opened_at)
    .bind(&input.emptied_at)
    .bind(input.weight_grams)
    .bind(input.diameter_mm)
    .bind(&input.notes)
    .fetch_one(&pool)
    .await
    .expect("insert failed");

    let spool = attach_filament(&pool, row).await;
    (StatusCode::OK, Json(SpoolResult { ok: true, error: None, spool: Some(spool) }))
}

async fn update(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<SpoolInput>,
) -> (StatusCode, Json<SpoolResult>) {
    if !dates_in_order(&input) {
        return err(StatusCode::BAD_REQUEST, DATES_OUT_OF_ORDER);
    }
    let row = sqlx::query_as::<_, SpoolRow>(
        "UPDATE spools SET lot_code = ?1, purchased_at = ?2, opened_at = ?3, emptied_at = ?4,
         weight_grams = ?5, diameter_mm = ?6, notes = ?7
         WHERE id = ?8
         RETURNING id, filament_id, lot_code, purchased_at, opened_at, emptied_at, weight_grams, diameter_mm, notes",
    )
    .bind(&input.lot_code)
    .bind(&input.purchased_at)
    .bind(&input.opened_at)
    .bind(&input.emptied_at)
    .bind(input.weight_grams)
    .bind(input.diameter_mm)
    .bind(&input.notes)
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("update failed");

    match row {
        Some(row) => {
            let spool = attach_filament(&pool, row).await;
            (StatusCode::OK, Json(SpoolResult { ok: true, error: None, spool: Some(spool) }))
        }
        None => err(StatusCode::NOT_FOUND, "not_found"),
    }
}

async fn delete(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<SpoolResult>) {
    let has_profiles = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM print_profiles WHERE spool_id = ?1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("query failed")
        > 0;
    if has_profiles {
        return err(StatusCode::BAD_REQUEST, "has_profiles");
    }

    let has_prints = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM prints WHERE spool_id = ?1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("query failed")
        > 0;
    if has_prints {
        return err(StatusCode::BAD_REQUEST, "has_prints");
    }

    let result = sqlx::query("DELETE FROM spools WHERE id = ?1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete failed");

    if result.rows_affected() == 0 {
        return err(StatusCode::NOT_FOUND, "not_found");
    }

    (StatusCode::OK, Json(SpoolResult { ok: true, error: None, spool: None }))
}
