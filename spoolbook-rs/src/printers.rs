use crate::printer_camera::CameraRegistry;
use crate::printer_mqtt::{self, ConnSupervisor, LiveStatusStore};
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;

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
        .route("/api/printers/{id}/live", get(live))
        .route("/api/printers/{id}/control", axum::routing::post(control))
        .route("/api/printers/test", axum::routing::post(test))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrinterConnectionTestRequest {
    ip_address: String,
    access_code: String,
}

async fn test(_editor: crate::auth::Editor, Json(req): Json<PrinterConnectionTestRequest>) -> Json<serde_json::Value> {
    match printer_mqtt::test_connection(&req.ip_address, &req.access_code).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(error) => Json(serde_json::json!({ "ok": false, "error": error })),
    }
}

#[derive(Deserialize)]
struct PrinterControlRequest {
    command: String,
}

#[derive(Serialize)]
struct PrinterControlResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn control(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Extension(store): Extension<LiveStatusStore>,
    Path(id): Path<i64>,
    Json(req): Json<PrinterControlRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let serial_number = sqlx::query_scalar::<_, Option<String>>("SELECT serial_number FROM printers WHERE id = ?1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .expect("query failed")
        .flatten();

    let Some(serial_number) = serial_number else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not_found" })));
    };

    if !["pause", "resume", "stop"].contains(&req.command.as_str()) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::to_value(PrinterControlResult { ok: false, error: Some("Unknown command.".into()) }).unwrap()));
    }

    match printer_mqtt::publish_command(&store, id, &serial_number, &req.command).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::to_value(PrinterControlResult { ok: true, error: None }).unwrap())),
        Err(error) => (StatusCode::BAD_REQUEST, Json(serde_json::to_value(PrinterControlResult { ok: false, error: Some(error) }).unwrap())),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrinterLiveSnapshot {
    connected: bool,
    ams_units: Vec<crate::bambu_mqtt_payload_parser::AmsUnitReading>,
    // Camera streaming (docs/adr/0024, ffmpeg/MJPEG relay) is a separate deferred slice — always
    // reports the pre-stream default rather than a real camera state.
    camera_status: &'static str,
    camera_error: Option<String>,
    gcode_state: Option<String>,
}

// SSE: direct port of PrinterCard.razor's poll loop (2s cadence). Streams a snapshot of the
// in-memory live-status store printer_mqtt.rs's background connection populates.
async fn live(
    Extension(store): Extension<LiveStatusStore>,
    Extension(camera_registry): Extension<crate::printer_camera::CameraRegistry>,
    Path(id): Path<i64>,
) -> Sse<impl tokio_stream::Stream<Item = Result<SseEvent, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(2))).then(move |_| {
        let store = store.clone();
        let camera_registry = camera_registry.clone();
        async move {
            let status = printer_mqtt::snapshot(&store, id).await;
            let camera_status = crate::printer_camera::status_of(&camera_registry, id).await;
            let camera_error = crate::printer_camera::last_error_of(&camera_registry, id).await;
            let snapshot = PrinterLiveSnapshot {
                connected: status.connected,
                ams_units: status.ams_units,
                camera_status: camera_status.as_str(),
                camera_error,
                gcode_state: status.gcode_state,
            };
            Ok(SseEvent::default().data(serde_json::to_string(&snapshot).unwrap()))
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub(crate) async fn get_by_id(pool: &SqlitePool, id: i64) -> Option<Printer> {
    sqlx::query_as::<_, Printer>("SELECT id, name, model, ip_address, access_code, serial_number FROM printers WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .expect("query failed")
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
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Extension(store): Extension<LiveStatusStore>,
    Extension(camera_registry): Extension<CameraRegistry>,
    Extension(supervisor): Extension<ConnSupervisor>,
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

    // Start its live-telemetry loop now, not just at the next restart's spawn_all — a printer
    // added while the process is already running is the norm under Docker.
    printer_mqtt::respawn_one(
        &supervisor,
        printer.id,
        printer.ip_address.clone(),
        printer.access_code.clone(),
        printer.serial_number.clone(),
        pool.clone(),
        store,
        camera_registry,
    )
    .await;

    (StatusCode::OK, Json(PrinterResult { ok: true, error: None, printer: Some(printer) }))
}

async fn update(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Extension(store): Extension<LiveStatusStore>,
    Extension(camera_registry): Extension<CameraRegistry>,
    Extension(supervisor): Extension<ConnSupervisor>,
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
        Some(printer) => {
            // Connection details may have changed — restart the loop against the new ones (and
            // drop it entirely if they were cleared). respawn_one aborts the stale loop first.
            printer_mqtt::respawn_one(
                &supervisor,
                printer.id,
                printer.ip_address.clone(),
                printer.access_code.clone(),
                printer.serial_number.clone(),
                pool.clone(),
                store,
                camera_registry,
            )
            .await;
            (StatusCode::OK, Json(PrinterResult { ok: true, error: None, printer: Some(printer) }))
        }
        None => err(StatusCode::NOT_FOUND, "not_found"),
    }
}

async fn delete(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Extension(store): Extension<LiveStatusStore>,
    Extension(supervisor): Extension<ConnSupervisor>,
    Path(id): Path<i64>,
) -> (StatusCode, Json<PrinterResult>) {
    let has_prints = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM prints WHERE printer_id = ?1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("query failed")
        > 0;
    if has_prints {
        return err(StatusCode::BAD_REQUEST, "has_prints");
    }

    let result = sqlx::query("DELETE FROM printers WHERE id = ?1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete failed");

    if result.rows_affected() == 0 {
        return err(StatusCode::NOT_FOUND, "not_found");
    }

    printer_mqtt::stop_one(&supervisor, &store, id).await;

    (StatusCode::OK, Json(PrinterResult { ok: true, error: None, printer: None }))
}
