use crate::profiles::PrintProfile;
use crate::projects::Project;
use crate::spools::Spool;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

// Weather auto-fetch (Open-Meteo, populating ambient_temp_c/ambient_humidity_pct/ambient_source)
// is deliberately not implemented here — unlike filament-catalog sync or project upload/import
// (both now ported), this one has no reference test suite to port forward, only a live external
// API. ActualRoomTempC (manual entry) already covers the non-network path.
#[derive(sqlx::FromRow)]
struct PrintRow {
    id: i64,
    profile_id: i64,
    spool_id: i64,
    printer_id: i64,
    project_id: Option<i64>,
    project_plater_id: Option<String>,
    started_at: String,
    ended_at: Option<String>,
    status: String,
    notes: Option<String>,
    ams_humidity_pct: Option<i64>,
    actual_room_temp_c: Option<f64>,
    clean_build_plate: Option<bool>,
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct PrinterSummary {
    id: i64,
    name: String,
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct PrintFailureModeEntry {
    mode: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Print {
    id: i64,
    profile_id: i64,
    profile: Option<PrintProfile>,
    printer_id: i64,
    printer: Option<PrinterSummary>,
    spool_id: i64,
    spool: Option<Spool>,
    project_id: Option<i64>,
    project: Option<Project>,
    project_plater_id: Option<String>,
    started_at: String,
    ended_at: Option<String>,
    status: String,
    notes: Option<String>,
    ams_humidity_pct: Option<i64>,
    actual_room_temp_c: Option<f64>,
    clean_build_plate: Option<bool>,
    failure_modes: Vec<PrintFailureModeEntry>,
}

async fn hydrate(pool: &SqlitePool, row: PrintRow) -> Print {
    let profile = sqlx::query_as::<_, PrintProfile>("SELECT * FROM print_profiles WHERE id = ?1")
        .bind(row.profile_id)
        .fetch_optional(pool)
        .await
        .expect("query failed");
    let spool = crate::spools::fetch_by_id(pool, row.spool_id).await;
    let printer = sqlx::query_as::<_, PrinterSummary>("SELECT id, name FROM printers WHERE id = ?1")
        .bind(row.printer_id)
        .fetch_optional(pool)
        .await
        .expect("query failed");
    let project = match row.project_id {
        Some(project_id) => sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?1")
            .bind(project_id)
            .fetch_optional(pool)
            .await
            .expect("query failed"),
        None => None,
    };
    let failure_modes = sqlx::query_as::<_, PrintFailureModeEntry>(
        "SELECT mode FROM print_failure_modes WHERE print_id = ?1",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await
    .expect("query failed");

    Print {
        id: row.id,
        profile_id: row.profile_id,
        profile,
        printer_id: row.printer_id,
        printer,
        spool_id: row.spool_id,
        spool,
        project_id: row.project_id,
        project,
        project_plater_id: row.project_plater_id,
        started_at: row.started_at,
        ended_at: row.ended_at,
        status: row.status,
        notes: row.notes,
        ams_humidity_pct: row.ams_humidity_pct,
        actual_room_temp_c: row.actual_room_temp_c,
        clean_build_plate: row.clean_build_plate,
        failure_modes,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintInputBody {
    started_at: String,
    ended_at: String,
    status: String,
    notes: Option<String>,
    ams_humidity_pct: Option<i64>,
    actual_room_temp_c: Option<f64>,
    clean_build_plate: Option<bool>,
    project_id: Option<i64>,
    project_plater_id: Option<String>,
    failure_modes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePrintBody {
    profile_id: i64,
    spool_id: i64,
    printer_id: i64,
    input: PrintInputBody,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePrintBody {
    printer_id: i64,
    input: PrintInputBody,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    print: Option<Print>,
}

fn err(status: StatusCode, message: &str) -> (StatusCode, Json<PrintResult>) {
    (status, Json(PrintResult { ok: false, error: Some(message.to_string()), print: None }))
}

fn dedup(modes: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    modes.iter().filter(|m| seen.insert((*m).clone())).cloned().collect()
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/prints", get(list).post(create))
        .route("/api/prints/inventory", get(inventory))
        .route("/api/prints/recommend-profile", get(recommend_profile))
        .route("/api/prints/job-match", get(job_match))
        .route("/api/prints/{id}", get(get_one).put(update).delete(delete))
        .route("/api/prints/{id}/attach-job", axum::routing::post(attach_job))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobMatchQuery {
    printer_id: i64,
    started_at: String,
}

async fn job_match(
    State(pool): State<SqlitePool>,
    Query(q): Query<JobMatchQuery>,
) -> Json<Option<crate::printer_telemetry::PrinterJob>> {
    Json(crate::printer_telemetry::find_match_for_print(&pool, q.printer_id, &q.started_at).await)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachJobRequest {
    job_id: i64,
}

async fn attach_job(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(req): Json<AttachJobRequest>,
) -> Json<serde_json::Value> {
    crate::printer_telemetry::attach_job_to_print(&pool, req.job_id, id).await;
    Json(serde_json::json!({ "ok": true }))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(rename = "printerId")]
    printer_id: Option<i64>,
}

async fn list(State(pool): State<SqlitePool>, Query(q): Query<ListQuery>) -> Json<Vec<Print>> {
    let sql = match q.printer_id {
        Some(_) => "SELECT * FROM prints WHERE printer_id = ?1 ORDER BY started_at DESC LIMIT 5",
        None => "SELECT * FROM prints ORDER BY started_at DESC",
    };
    let mut query = sqlx::query_as::<_, PrintRow>(sql);
    if let Some(printer_id) = q.printer_id {
        query = query.bind(printer_id);
    }
    let rows = query.fetch_all(&pool).await.expect("query failed");

    let mut prints = Vec::with_capacity(rows.len());
    for row in rows {
        prints.push(hydrate(&pool, row).await);
    }
    Json(prints)
}

async fn get_one(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Result<Json<Print>, StatusCode> {
    let row = sqlx::query_as::<_, PrintRow>("SELECT * FROM prints WHERE id = ?1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .expect("query failed");

    match row {
        Some(row) => Ok(Json(hydrate(&pool, row).await)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
struct InventoryQuery {
    status: Option<String>,
    #[serde(rename = "printerId")]
    printer_id: Option<i64>,
    page: Option<i64>,
    #[serde(rename = "pageSize")]
    page_size: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrintInventoryResult {
    prints: Vec<Print>,
    total: i64,
    page: i64,
    page_size: i64,
    total_pages: i64,
}

async fn inventory(State(pool): State<SqlitePool>, Query(q): Query<InventoryQuery>) -> Json<PrintInventoryResult> {
    let page = q.page.filter(|&p| p > 0).unwrap_or(1);
    let page_size = q.page_size.filter(|&s| s > 0).unwrap_or(20);
    let status = q.status.filter(|s| !s.is_empty());

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM prints WHERE (?1 IS NULL OR status = ?1) AND (?2 IS NULL OR printer_id = ?2)",
    )
    .bind(&status)
    .bind(q.printer_id)
    .fetch_one(&pool)
    .await
    .expect("query failed");

    let rows = sqlx::query_as::<_, PrintRow>(
        "SELECT * FROM prints WHERE (?1 IS NULL OR status = ?1) AND (?2 IS NULL OR printer_id = ?2)
         ORDER BY started_at DESC LIMIT ?3 OFFSET ?4",
    )
    .bind(&status)
    .bind(q.printer_id)
    .bind(page_size)
    .bind((page - 1) * page_size)
    .fetch_all(&pool)
    .await
    .expect("query failed");

    let mut prints = Vec::with_capacity(rows.len());
    for row in rows {
        prints.push(hydrate(&pool, row).await);
    }

    let total_pages = ((total as f64) / (page_size as f64)).ceil().max(1.0) as i64;
    Json(PrintInventoryResult { prints, total, page, page_size, total_pages })
}

#[derive(Deserialize)]
struct RecommendQuery {
    #[serde(rename = "projectId")]
    project_id: i64,
    #[serde(rename = "currentTempC")]
    current_temp_c: Option<f64>,
}

// Ranked by Status (Success > Partial > Failed > InProgress), ties broken by closest ambient
// match (ActualRoomTempC), matching PrintService.RecommendProfileForProjectAsync.
async fn recommend_profile(State(pool): State<SqlitePool>, Query(q): Query<RecommendQuery>) -> Json<Option<PrintProfile>> {
    let rows = sqlx::query_as::<_, PrintRow>("SELECT * FROM prints WHERE project_id = ?1")
        .bind(q.project_id)
        .fetch_all(&pool)
        .await
        .expect("query failed");

    fn status_rank(status: &str) -> i32 {
        match status {
            "Success" => 0,
            "Partial" => 1,
            "Failed" => 2,
            _ => 3,
        }
    }
    fn temp_distance(row: &PrintRow, current: Option<f64>) -> f64 {
        match (row.actual_room_temp_c, current) {
            (Some(effective), Some(current)) => (effective - current).abs(),
            _ => f64::MAX,
        }
    }

    let best = rows
        .iter()
        .min_by(|a, b| {
            status_rank(&a.status)
                .cmp(&status_rank(&b.status))
                .then(temp_distance(a, q.current_temp_c).partial_cmp(&temp_distance(b, q.current_temp_c)).unwrap())
                .then(b.started_at.cmp(&a.started_at))
        });

    let profile = match best {
        Some(row) => sqlx::query_as::<_, PrintProfile>("SELECT * FROM print_profiles WHERE id = ?1")
            .bind(row.profile_id)
            .fetch_optional(&pool)
            .await
            .expect("query failed"),
        None => None,
    };

    Json(profile)
}

// Auto-create-on-send (docs/adr/0017's 2026-08-14 addendum) — called the moment the printer
// send succeeds, so the Print row exists before the printer even confirms it started.
// EndedAt/ambient weather fill in later (printer_telemetry::end_job / a future weather fetch).
pub(crate) async fn create_in_progress(
    pool: &SqlitePool,
    profile_id: i64,
    spool_id: i64,
    printer_id: i64,
    project_id: Option<i64>,
    project_plater_id: Option<&str>,
    started_at: &str,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO prints (profile_id, spool_id, printer_id, project_id, project_plater_id, started_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'InProgress')
         RETURNING id",
    )
    .bind(profile_id)
    .bind(spool_id)
    .bind(printer_id)
    .bind(project_id)
    .bind(project_plater_id)
    .bind(started_at)
    .fetch_one(pool)
    .await
    .expect("insert failed")
}

async fn create(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Json(body): Json<CreatePrintBody>,
) -> (StatusCode, Json<PrintResult>) {
    let modes = dedup(&body.input.failure_modes);
    if !modes.is_empty() && body.input.status == "Success" {
        return err(StatusCode::BAD_REQUEST, "failure_modes_require_failed_or_partial");
    }

    let mut tx = pool.begin().await.expect("begin failed");

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO prints (
            profile_id, spool_id, printer_id, project_id, project_plater_id,
            started_at, ended_at, status, notes, ams_humidity_pct, actual_room_temp_c, clean_build_plate
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        RETURNING id",
    )
    .bind(body.profile_id)
    .bind(body.spool_id)
    .bind(body.printer_id)
    .bind(body.input.project_id)
    .bind(&body.input.project_plater_id)
    .bind(&body.input.started_at)
    .bind(&body.input.ended_at)
    .bind(&body.input.status)
    .bind(&body.input.notes)
    .bind(body.input.ams_humidity_pct)
    .bind(body.input.actual_room_temp_c)
    .bind(body.input.clean_build_plate)
    .fetch_one(&mut *tx)
    .await
    .expect("insert failed");

    for mode in &modes {
        sqlx::query("INSERT INTO print_failure_modes (print_id, mode) VALUES (?1, ?2)")
            .bind(id)
            .bind(mode)
            .execute(&mut *tx)
            .await
            .expect("insert failed");
    }

    tx.commit().await.expect("commit failed");

    let row = sqlx::query_as::<_, PrintRow>("SELECT * FROM prints WHERE id = ?1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    let print = hydrate(&pool, row).await;

    (StatusCode::OK, Json(PrintResult { ok: true, error: None, print: Some(print) }))
}

async fn update(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(body): Json<UpdatePrintBody>,
) -> (StatusCode, Json<PrintResult>) {
    let modes = dedup(&body.input.failure_modes);
    if !modes.is_empty() && body.input.status == "Success" {
        return err(StatusCode::BAD_REQUEST, "failure_modes_require_failed_or_partial");
    }

    let mut tx = pool.begin().await.expect("begin failed");

    let updated = sqlx::query(
        "UPDATE prints SET printer_id = ?1, project_id = ?2, project_plater_id = ?3, started_at = ?4,
         ended_at = ?5, status = ?6, notes = ?7, ams_humidity_pct = ?8, actual_room_temp_c = ?9, clean_build_plate = ?10
         WHERE id = ?11",
    )
    .bind(body.printer_id)
    .bind(body.input.project_id)
    .bind(&body.input.project_plater_id)
    .bind(&body.input.started_at)
    .bind(&body.input.ended_at)
    .bind(&body.input.status)
    .bind(&body.input.notes)
    .bind(body.input.ams_humidity_pct)
    .bind(body.input.actual_room_temp_c)
    .bind(body.input.clean_build_plate)
    .bind(id)
    .execute(&mut *tx)
    .await
    .expect("update failed");

    if updated.rows_affected() == 0 {
        return err(StatusCode::NOT_FOUND, "not_found");
    }

    sqlx::query("DELETE FROM print_failure_modes WHERE print_id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .expect("delete failed");
    for mode in &modes {
        sqlx::query("INSERT INTO print_failure_modes (print_id, mode) VALUES (?1, ?2)")
            .bind(id)
            .bind(mode)
            .execute(&mut *tx)
            .await
            .expect("insert failed");
    }

    tx.commit().await.expect("commit failed");

    let row = sqlx::query_as::<_, PrintRow>("SELECT * FROM prints WHERE id = ?1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    let print = hydrate(&pool, row).await;

    (StatusCode::OK, Json(PrintResult { ok: true, error: None, print: Some(print) }))
}

async fn delete(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<PrintResult>) {
    let result = sqlx::query("DELETE FROM prints WHERE id = ?1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete failed");

    if result.rows_affected() == 0 {
        return err(StatusCode::NOT_FOUND, "not_found");
    }

    (StatusCode::OK, Json(PrintResult { ok: true, error: None, print: None }))
}
