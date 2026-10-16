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

// ams_humidity_pct/chamber_temp_c are auto-snapshotted from the printer's own live MQTT reading
// at print-end (printer_mqtt.rs's handle_message -> printer_telemetry::end_job), same as
// ambient_temp_c/ambient_humidity_pct/ambient_source are auto-fetched from Open-Meteo there —
// none of the five are client-writable, so none appear in PrintInputBody.
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
    chamber_temp_c: Option<f64>,
    actual_room_temp_c: Option<f64>,
    clean_build_plate: Option<bool>,
    ambient_temp_c: Option<f64>,
    ambient_humidity_pct: Option<f64>,
    ambient_source: Option<String>,
    bed_photo_base64: Option<String>,
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
    chamber_temp_c: Option<f64>,
    // Hand-entered when annotating the outcome (FR-013); chamber_temp_c is telemetry.
    actual_room_temp_c: Option<f64>,
    clean_build_plate: Option<bool>,
    // Auto-fetched only (issues/94) -- never client-writable, so these have no counterpart in
    // PrintInputBody.
    ambient_temp_c: Option<f64>,
    ambient_humidity_pct: Option<f64>,
    ambient_source: Option<String>,
    // Auto-captured at end-of-print regardless of terminal status (issues/121) -- never
    // client-writable either.
    bed_photo_base64: Option<String>,
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
        chamber_temp_c: row.chamber_temp_c,
        actual_room_temp_c: row.actual_room_temp_c,
        clean_build_plate: row.clean_build_plate,
        ambient_temp_c: row.ambient_temp_c,
        ambient_humidity_pct: row.ambient_humidity_pct,
        ambient_source: row.ambient_source,
        bed_photo_base64: row.bed_photo_base64,
        failure_modes,
    }
}

// The only writable part of a Print: its outcome, annotated once it's finished (003/FR-013).
// Anything else in the request is ignored — a Print's settings come from the actual print.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeBody {
    status: String,
    #[serde(default)]
    failure_modes: Vec<String>,
    notes: Option<String>,
    actual_room_temp_c: Option<f64>,
    clean_build_plate: Option<bool>,
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

// InProgress is telemetry's to set and clear, never an annotation.
const OUTCOMES: [&str; 3] = ["Success", "Failed", "Partial"];
const FAILURE_MODES: [&str; 8] = ["Stringing", "LayerAdhesion", "Warping", "UnderExtrusion", "OverExtrusion", "LayerShift", "Clog", "Other"];

fn validate(outcome: &OutcomeBody, modes: &[String]) -> Result<(), String> {
    if !OUTCOMES.contains(&outcome.status.as_str()) {
        return Err("Unknown status.".into());
    }
    if let Some(mode) = modes.iter().find(|m| !FAILURE_MODES.contains(&m.as_str())) {
        return Err(format!("Unknown failure mode: {mode}."));
    }
    if !modes.is_empty() && outcome.status == "Success" {
        return Err("failure_modes_require_failed_or_partial".into());
    }
    Ok(())
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/prints", get(list))
        .route("/api/prints/inventory", get(inventory))
        .route("/api/prints/recommend-profile", get(recommend_profile))
        .route("/api/prints/{id}", get(get_one).put(update).delete(delete))
        .route("/api/prints/{id}/hourly-weather", get(hourly_weather))
        .route("/api/prints/{id}/readings", get(readings))
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct HourlyWeatherRow {
    hour: String,
    temp_c: Option<f64>,
    humidity_pct: Option<f64>,
}

async fn hourly_weather(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Json<Vec<HourlyWeatherRow>> {
    let rows = sqlx::query_as::<_, HourlyWeatherRow>(
        "SELECT hour, temp_c, humidity_pct FROM print_hourly_weather WHERE print_id = ?1 ORDER BY hour",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .expect("query failed");
    Json(rows)
}

async fn readings(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Json<Vec<crate::printer_telemetry::ReadingSnapshot>> {
    let json: Option<String> = sqlx::query_scalar("SELECT telemetry_json FROM prints WHERE id = ?1").bind(id).fetch_optional(&pool).await.expect("query failed").flatten();
    Json(json.map(|j| serde_json::from_str(&j).expect("stored telemetry_json is always valid")).unwrap_or_default())
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(rename = "printerId")]
    printer_id: Option<i64>,
    #[serde(rename = "projectId")]
    project_id: Option<i64>,
}

async fn list(State(pool): State<SqlitePool>, Query(q): Query<ListQuery>) -> Json<Vec<Print>> {
    let sql = match (q.printer_id, q.project_id) {
        (Some(_), _) => "SELECT * FROM prints WHERE printer_id = ?1 ORDER BY started_at DESC LIMIT 5",
        (None, Some(_)) => "SELECT * FROM prints WHERE project_id = ?1 ORDER BY started_at DESC",
        (None, None) => "SELECT * FROM prints ORDER BY started_at DESC",
    };
    let mut query = sqlx::query_as::<_, PrintRow>(sql);
    if let Some(printer_id) = q.printer_id {
        query = query.bind(printer_id);
    } else if let Some(project_id) = q.project_id {
        query = query.bind(project_id);
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
    // Ranked against ambient_temp_c (Open-Meteo, auto-fetched at print-end) rather than
    // chamber_temp_c — chamber is the printer enclosure's own heat during the print, not a
    // stand-in for outside/room conditions, so it's the wrong axis for "closest weather match".
    fn temp_distance(row: &PrintRow, current: Option<f64>) -> f64 {
        match (row.ambient_temp_c, current) {
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

// Auto-create-on-send — called the moment the printer
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

async fn update(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(body): Json<OutcomeBody>,
) -> (StatusCode, Json<PrintResult>) {
    let modes = dedup(&body.failure_modes);
    if let Err(e) = validate(&body, &modes) {
        return err(StatusCode::BAD_REQUEST, &e);
    }
    let current: Option<String> = sqlx::query_scalar("SELECT status FROM prints WHERE id = ?1").bind(id).fetch_optional(&pool).await.expect("query failed");
    match current.as_deref() {
        None => return err(StatusCode::NOT_FOUND, "not_found"),
        Some("InProgress") => return err(StatusCode::BAD_REQUEST, "still_in_progress"),
        Some(_) => {}
    }

    let mut tx = pool.begin().await.expect("begin failed");
    sqlx::query("UPDATE prints SET status = ?1, notes = ?2, actual_room_temp_c = ?3, clean_build_plate = ?4 WHERE id = ?5")
        .bind(&body.status)
        .bind(&body.notes)
        .bind(body.actual_room_temp_c)
        .bind(body.clean_build_plate)
        .bind(id)
        .execute(&mut *tx)
        .await
        .expect("update failed");
    sqlx::query("DELETE FROM print_failure_modes WHERE print_id = ?1").bind(id).execute(&mut *tx).await.expect("delete failed");
    for mode in &modes {
        sqlx::query("INSERT INTO print_failure_modes (print_id, mode) VALUES (?1, ?2)").bind(id).bind(mode).execute(&mut *tx).await.expect("insert failed");
    }
    tx.commit().await.expect("commit failed");

    let row = sqlx::query_as::<_, PrintRow>("SELECT * FROM prints WHERE id = ?1").bind(id).fetch_one(&pool).await.expect("query failed");
    let print = hydrate(&pool, row).await;
    (StatusCode::OK, Json(PrintResult { ok: true, error: None, print: Some(print) }))
}

async fn delete(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<PrintResult>) {
    // print_failure_modes.print_id has no ON DELETE CASCADE (hourly weather cascades, attached
    // Jobs are SET NULL), so the tags go first — same transaction, so a failure keeps both.
    // ponytail: explicit delete instead of a table-rebuild migration to add the cascade; this is
    // the only place Prints are deleted.
    let mut tx = pool.begin().await.expect("begin failed");
    sqlx::query("DELETE FROM print_failure_modes WHERE print_id = ?1").bind(id).execute(&mut *tx).await.expect("delete failed");
    let result = sqlx::query("DELETE FROM prints WHERE id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .expect("delete failed");

    if result.rows_affected() == 0 {
        return err(StatusCode::NOT_FOUND, "not_found");
    }
    tx.commit().await.expect("commit failed");

    (StatusCode::OK, Json(PrintResult { ok: true, error: None, print: None }))
}
