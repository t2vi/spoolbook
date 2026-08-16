use crate::filaments::Filament;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, State},
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

async fn get_one(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Result<Json<Spool>, StatusCode> {
    let row = sqlx::query_as::<_, SpoolRow>(
        "SELECT id, filament_id, lot_code, purchased_at, opened_at, emptied_at, weight_grams, diameter_mm, notes
         FROM spools WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("query failed");

    match row {
        Some(row) => Ok(Json(attach_filament(&pool, row).await)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

fn err(status: StatusCode, message: &str) -> (StatusCode, Json<SpoolResult>) {
    (status, Json(SpoolResult { ok: false, error: Some(message.to_string()), spool: None }))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(body): Json<CreateSpoolInput>,
) -> (StatusCode, Json<SpoolResult>) {
    let input = body.input;
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
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<SpoolInput>,
) -> (StatusCode, Json<SpoolResult>) {
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

// .NET also blocks deleting a Spool referenced by a PrintProfile or a Print (has_profiles /
// has_prints). Neither table exists in this DB yet — no Profiles/Prints slice ported — so this
// unconditionally deletes. Add those guards back once those slices land.
async fn delete(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<SpoolResult>) {
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
