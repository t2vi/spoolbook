use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use spoolbook_rs::printer_mqtt::{handle_message, new_store, snapshot};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tokio_stream::StreamExt;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn seed_printer(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("INSERT INTO printers (name) VALUES ('Garage P2S') RETURNING id").fetch_one(pool).await.unwrap()
}

fn running_payload(task_id: &str) -> String {
    format!(
        r##"{{"print":{{"gcode_state":"RUNNING","task_id":"{task_id}","nozzle_temper":240.0,"bed_temper":70.0,"mc_percent":8}}}}"##
    )
}

fn finish_payload() -> &'static str {
    r#"{"print":{"gcode_state":"FINISH"}}"#
}

#[tokio::test]
async fn handle_message_records_a_reading_and_updates_the_live_store_on_active_state() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let store = new_store();
    let mut active_task_id = None;

    handle_message(printer_id, &running_payload("1725"), &pool, &store, &mut active_task_id).await;

    assert_eq!(active_task_id.as_deref(), Some("1725"));
    let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert_eq!(job_count, 1);

    let status = snapshot(&store, printer_id).await;
    assert!(status.connected);
    assert_eq!(status.gcode_state.as_deref(), Some("RUNNING"));
}

#[tokio::test]
async fn handle_message_ends_the_active_job_on_transition_to_terminal_state() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let store = new_store();
    let mut active_task_id = None;
    handle_message(printer_id, &running_payload("1725"), &pool, &store, &mut active_task_id).await;

    handle_message(printer_id, finish_payload(), &pool, &store, &mut active_task_id).await;

    assert_eq!(active_task_id, None, "consumed on end, so a repeat terminal message can't double-end");
    let ended_at: Option<String> = sqlx::query_scalar("SELECT ended_at FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert!(ended_at.is_some());

    let status = snapshot(&store, printer_id).await;
    assert_eq!(status.gcode_state.as_deref(), Some("FINISH"));
}

#[tokio::test]
async fn handle_message_does_not_end_a_job_twice_on_repeated_terminal_messages() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let store = new_store();
    let mut active_task_id = None;
    handle_message(printer_id, &running_payload("1725"), &pool, &store, &mut active_task_id).await;
    handle_message(printer_id, finish_payload(), &pool, &store, &mut active_task_id).await;

    // A second terminal message (e.g. IDLE right after FINISH) with no active job tracked
    // anymore must be a no-op, not an error or a second end_job call.
    handle_message(printer_id, r#"{"print":{"gcode_state":"IDLE"}}"#, &pool, &store, &mut active_task_id).await;

    assert_eq!(active_task_id, None);
}

#[tokio::test]
async fn handle_message_ignores_unparseable_payloads() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let store = new_store();
    let mut active_task_id = None;

    handle_message(printer_id, "not json", &pool, &store, &mut active_task_id).await;

    let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert_eq!(job_count, 0);
    let status = snapshot(&store, printer_id).await;
    assert!(!status.connected);
}

#[tokio::test]
async fn handle_message_does_not_overwrite_ams_units_with_an_empty_update() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let store = new_store();
    let mut active_task_id = None;
    let with_ams = r#"{"print":{"gcode_state":"RUNNING","task_id":"1","ams":{"ams":[{"id":"0","humidity":"5","tray":[]}]}}}"#;
    handle_message(printer_id, with_ams, &pool, &store, &mut active_task_id).await;

    // A follow-up message with no "ams" key at all (common for delta updates) must not clear
    // the AMS inventory we already know about.
    handle_message(printer_id, &running_payload("1"), &pool, &store, &mut active_task_id).await;

    let status = snapshot(&store, printer_id).await;
    assert_eq!(status.ams_units.len(), 1);
}

// The SSE stream never ends (2s interval, forever) — draining it with axum::body::to_bytes
// would hang the test. Pull just the first chunk instead, then drop the response (which cancels
// the underlying stream task) — enough to prove the store's snapshot reaches the wire correctly.
async fn send_sse(pool: &sqlx::SqlitePool, store: &spoolbook_rs::printer_mqtt::LiveStatusStore, uri: &str) -> (StatusCode, String) {
    let response = spoolbook_rs::app_with_live_status(pool.clone(), store.clone())
        .oneshot(Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();

    let mut stream = response.into_body().into_data_stream();
    let first_chunk = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .expect("timed out waiting for the first SSE chunk")
        .expect("stream ended with no data")
        .unwrap();
    (status, String::from_utf8(first_chunk.to_vec()).unwrap())
}

#[tokio::test]
async fn live_endpoint_streams_the_stores_snapshot_for_the_requested_printer() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let store = new_store();
    let mut active_task_id = None;
    handle_message(printer_id, &running_payload("1725"), &pool, &store, &mut active_task_id).await;

    let (status, body) = send_sse(&pool, &store, &format!("/api/printers/{printer_id}/live")).await;

    assert_eq!(status, StatusCode::OK);
    let data_line = body.lines().find(|l| l.starts_with("data:")).expect("no data: line in SSE body");
    let json: Value = serde_json::from_str(data_line.trim_start_matches("data:").trim()).unwrap();
    assert_eq!(json["connected"], true);
    assert_eq!(json["gcodeState"], "RUNNING");
    assert_eq!(json["cameraStatus"], "NotStarted");
}

#[tokio::test]
async fn live_endpoint_reports_disconnected_for_a_printer_with_no_live_status() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let store = new_store();

    let (status, body) = send_sse(&pool, &store, &format!("/api/printers/{printer_id}/live")).await;

    assert_eq!(status, StatusCode::OK);
    let data_line = body.lines().find(|l| l.starts_with("data:")).unwrap();
    let json: Value = serde_json::from_str(data_line.trim_start_matches("data:").trim()).unwrap();
    assert_eq!(json["connected"], false);
    assert_eq!(json["gcodeState"], Value::Null);
}

