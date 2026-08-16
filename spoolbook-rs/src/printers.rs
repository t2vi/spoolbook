use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Printer {
    pub id: i64,
    pub name: String,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub access_code: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInput {
    pub name: String,
    pub model: Option<String>,
    pub ip_address: Option<String>,
    pub access_code: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printer: Option<Printer>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/printers", get(list).post(create))
        .route("/api/printers/{id}", axum::routing::put(update).delete(delete))
}

async fn list(State(pool): State<SqlitePool>) -> Json<Vec<Printer>> {
    let printers = sqlx::query_as::<_, Printer>(
        "SELECT id, name, model, ip_address, access_code, serial_number FROM printers ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .expect("query failed");

    Json(printers)
}

fn err(status: StatusCode, message: &str) -> (StatusCode, Json<PrinterResult>) {
    (status, Json(PrinterResult { ok: false, error: Some(message.to_string()), printer: None }))
}

async fn is_duplicate(pool: &SqlitePool, name: &str, exclude_id: Option<i64>) -> bool {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM printers WHERE name = ?1 AND (?2 IS NULL OR id != ?2)",
    )
    .bind(name)
    .bind(exclude_id)
    .fetch_one(pool)
    .await
    .expect("query failed");

    count > 0
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(input): Json<PrinterInput>,
) -> (StatusCode, Json<PrinterResult>) {
    if input.name.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "Name is required");
    }
    if is_duplicate(&pool, &input.name, None).await {
        return err(StatusCode::BAD_REQUEST, "duplicate");
    }

    let printer = sqlx::query_as::<_, Printer>(
        "INSERT INTO printers (name, model, ip_address, access_code, serial_number) VALUES (?1, ?2, ?3, ?4, ?5)
         RETURNING id, name, model, ip_address, access_code, serial_number",
    )
    .bind(&input.name)
    .bind(&input.model)
    .bind(&input.ip_address)
    .bind(&input.access_code)
    .bind(&input.serial_number)
    .fetch_one(&pool)
    .await
    .expect("insert failed");

    (StatusCode::OK, Json(PrinterResult { ok: true, error: None, printer: Some(printer) }))
}

async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<PrinterInput>,
) -> (StatusCode, Json<PrinterResult>) {
    if input.name.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "Name is required");
    }
    if is_duplicate(&pool, &input.name, Some(id)).await {
        return err(StatusCode::BAD_REQUEST, "duplicate");
    }

    let printer = sqlx::query_as::<_, Printer>(
        "UPDATE printers SET name = ?1, model = ?2, ip_address = ?3, access_code = ?4, serial_number = ?5
         WHERE id = ?6
         RETURNING id, name, model, ip_address, access_code, serial_number",
    )
    .bind(&input.name)
    .bind(&input.model)
    .bind(&input.ip_address)
    .bind(&input.access_code)
    .bind(&input.serial_number)
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("update failed");

    match printer {
        Some(printer) => (StatusCode::OK, Json(PrinterResult { ok: true, error: None, printer: Some(printer) })),
        None => err(StatusCode::NOT_FOUND, "not_found"),
    }
}

// .NET also blocks deleting a printer referenced by a Print (has_prints) — Prints table
// doesn't exist in this DB yet. Same gap as Spool/PrintProfile's delete guards.
async fn delete(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<PrinterResult>) {
    let result = sqlx::query("DELETE FROM printers WHERE id = ?1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete failed");

    if result.rows_affected() == 0 {
        return err(StatusCode::NOT_FOUND, "not_found");
    }

    (StatusCode::OK, Json(PrinterResult { ok: true, error: None, printer: None }))
}
