#![recursion_limit = "512"]
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to open in-memory db");
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn send(pool: &sqlx::SqlitePool, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", common::auth_cookie_header(pool).await)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

async fn seed_filament(pool: &sqlx::SqlitePool) -> i64 {
    let (_, body) = send(pool, "POST", "/api/filaments", Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }))).await;
    body["entry"]["id"].as_i64().unwrap()
}

async fn seed_spool(pool: &sqlx::SqlitePool, filament_id: i64) -> i64 {
    let (_, body) = send(pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    body["spool"]["id"].as_i64().unwrap()
}

async fn seed_printer(pool: &sqlx::SqlitePool, name: &str) -> i64 {
    let (_, body) = send(pool, "POST", "/api/printers", Some(json!({
        "name": name, "model": "P2S", "ipAddress": null, "accessCode": null, "serialNumber": null
    }))).await;
    body["printer"]["id"].as_i64().unwrap()
}

// Minimal profile seed — reuses the DB directly rather than the full ~135-field JSON body
// (already stress-tested in tests/profiles.rs); only nozzleTempC is required.
async fn seed_profile(pool: &sqlx::SqlitePool, filament_id: i64, name: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, ?2, 220) RETURNING id")
        .bind(filament_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_project(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO projects (file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes)
         VALUES ('/tmp/x.3mf', 'x.3mf', '2026-01-01T00:00:00Z', 100) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

struct Fixtures {
    filament_id: i64,
    profile_id: i64,
    spool_id: i64,
    printer_id: i64,
}

async fn seed_all(pool: &sqlx::SqlitePool) -> Fixtures {
    let filament_id = seed_filament(pool).await;
    let spool_id = seed_spool(pool, filament_id).await;
    let printer_id = seed_printer(pool, "P2S #1").await;
    let profile_id = seed_profile(pool, filament_id, "Default").await;
    Fixtures { filament_id, profile_id, spool_id, printer_id }
}

fn print_body(f: &Fixtures, status: &str, failure_modes: Vec<&str>) -> Value {
    json!({
        "profileId": f.profile_id, "spoolId": f.spool_id, "printerId": f.printer_id,
        "input": {
            "startedAt": "2026-08-15T10:00:00Z", "endedAt": "2026-08-15T12:00:00Z",
            "status": status, "notes": "test print", "cleanBuildPlate": true,
            "projectId": null, "projectPlaterId": null,
            "failureModes": failure_modes
        }
    })
}

// spec: 003/FR-001
#[tokio::test]
async fn get_by_id_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "GET", "/api/prints/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// spec: 003/FR-001
#[tokio::test]
async fn get_by_id_returns_the_full_nested_print() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let id = created["print"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "GET", &format!("/api/prints/{id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
    assert_eq!(body["profile"]["nozzleTempC"], 220);
}

// spec: 003/FR-002
#[tokio::test]
async fn list_filters_by_printer_and_caps_at_five() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    for _ in 0..6 {
        common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    }
    let other_printer_id = seed_printer(&pool, "P2S #2").await;
    let cross_printer = Fixtures { filament_id: f.filament_id, profile_id: f.profile_id, spool_id: f.spool_id, printer_id: other_printer_id };
    common::create_print(&pool, Some(print_body(&cross_printer, "Success", vec![]))).await;

    let (status, body) = send(&pool, "GET", &format!("/api/prints?printerId={}", f.printer_id), None).await;

    assert_eq!(status, StatusCode::OK);
    let prints = body.as_array().unwrap();
    assert_eq!(prints.len(), 5, "capped at 5 even though 6 were created for this printer");
    assert!(prints.iter().all(|p| p["printerId"] == f.printer_id), "excludes the other printer's print");
}

// spec: 003/FR-002
#[tokio::test]
async fn list_filters_by_project_id() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let project_id = seed_project(&pool).await;
    let mut body = print_body(&f, "Success", vec![]);
    body["input"]["projectId"] = json!(project_id);
    body["input"]["projectPlaterId"] = json!("1");
    common::create_print(&pool, Some(body)).await;
    common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;

    let (status, body) = send(&pool, "GET", &format!("/api/prints?projectId={project_id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    let prints = body.as_array().unwrap();
    assert_eq!(prints.len(), 1, "excludes the print with no project");
    assert_eq!(prints[0]["projectId"], project_id);
}

// spec: 003/FR-002
#[tokio::test]
async fn list_without_printer_id_returns_everything() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;

    let (status, body) = send(&pool, "GET", "/api/prints", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

// spec: 003/FR-003
#[tokio::test]
async fn inventory_filters_by_status_and_paginates() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    common::create_print(&pool, Some(print_body(&f, "Failed", vec!["Clog"]))).await;

    let (status, body) = send(&pool, "GET", "/api/prints/inventory?status=Success&page=1&pageSize=20", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["prints"][0]["status"], "Success");
}

// spec: 003/FR-020
#[tokio::test]
async fn recommend_profile_picks_success_over_failed() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let project_id = seed_project(&pool).await;

    let mut failed_body = print_body(&f, "Failed", vec!["Warping"]);
    failed_body["input"]["projectId"] = json!(project_id);
    common::create_print(&pool, Some(failed_body)).await;

    let success_profile_id = seed_profile(&pool, f.filament_id, "Success Profile").await;
    let success_f = Fixtures { filament_id: f.filament_id, profile_id: success_profile_id, spool_id: f.spool_id, printer_id: f.printer_id };
    let mut success_body = print_body(&success_f, "Success", vec![]);
    success_body["input"]["projectId"] = json!(project_id);
    common::create_print(&pool, Some(success_body)).await;

    let (status, body) = send(&pool, "GET", &format!("/api/prints/recommend-profile?projectId={project_id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Success Profile");
}

// spec: 003/FR-020
#[tokio::test]
async fn recommend_profile_returns_null_when_no_prints_reference_the_project() {
    let pool = test_pool().await;
    let project_id = seed_project(&pool).await;
    let (status, body) = send(&pool, "GET", &format!("/api/prints/recommend-profile?projectId={project_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Null);
}

// spec: 003/FR-014
#[tokio::test]
async fn delete_removes_the_print() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let id = created["print"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "DELETE", &format!("/api/prints/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    let (_, all) = send(&pool, "GET", "/api/prints", None).await;
    assert_eq!(all.as_array().unwrap().len(), 0);
}

// spec: 003/FR-014
#[tokio::test]
async fn delete_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "DELETE", "/api/prints/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn seed_printer_job(pool: &sqlx::SqlitePool, printer_id: i64, external_job_id: &str, started_at: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO printer_jobs (printer_id, external_job_id, started_at) VALUES (?1, ?2, ?3) RETURNING id",
    )
    .bind(printer_id)
    .bind(external_job_id)
    .bind(started_at)
    .fetch_one(pool)
    .await
    .unwrap()
}

// spec: 003/FR-004
#[tokio::test]
async fn hourly_weather_returns_rows_ordered_by_hour() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let print_id = created["print"]["id"].as_i64().unwrap();
    sqlx::query("INSERT INTO print_hourly_weather (print_id, hour, temp_c, humidity_pct) VALUES (?1, '2026-01-01T08:00', 22.0, 52.0), (?1, '2026-01-01T07:00', 20.0, 50.0)")
        .bind(print_id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = send(&pool, "GET", &format!("/api/prints/{print_id}/hourly-weather"), None).await;

    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["hour"], "2026-01-01T07:00");
    assert_eq!(rows[0]["tempC"], 20.0);
    assert_eq!(rows[1]["hour"], "2026-01-01T08:00");
}

// spec: 003/FR-005
#[tokio::test]
async fn readings_returns_the_stored_telemetry_snapshot() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let print_id = created["print"]["id"].as_i64().unwrap();
    let json = r#"[{"recordedAt":"2026-01-01T08:00:10Z","chamberTempC":35.0,"amsHumidityPct":38,"layerNum":6,"totalLayerNum":908,"progressPct":8}]"#;
    sqlx::query("UPDATE prints SET telemetry_json = ?1 WHERE id = ?2").bind(json).bind(print_id).execute(&pool).await.unwrap();

    let (status, body) = send(&pool, "GET", &format!("/api/prints/{print_id}/readings"), None).await;

    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["chamberTempC"], 35.0);
    assert_eq!(rows[0]["layerNum"], 6);
}

// spec: 003/FR-005
#[tokio::test]
async fn readings_returns_empty_array_when_no_snapshot_exists() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let print_id = created["print"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "GET", &format!("/api/prints/{print_id}/readings"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// spec: 003/FR-004
#[tokio::test]
async fn hourly_weather_returns_empty_array_when_none_recorded() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let print_id = created["print"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "GET", &format!("/api/prints/{print_id}/hourly-weather"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// spec: 003/FR-014
#[tokio::test]
async fn delete_removes_a_print_with_failure_modes_weather_and_an_attached_job() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Failed", vec!["Stringing", "Warping"]))).await;
    let id = created["print"]["id"].as_i64().unwrap();
    sqlx::query("INSERT INTO print_hourly_weather (print_id, hour, temp_c, humidity_pct) VALUES (?1, '2026-08-15T10:00', 18.0, 60.0)").bind(id).execute(&pool).await.unwrap();
    let job_id = seed_printer_job(&pool, f.printer_id, "task-1", "2026-08-15T10:00:00Z").await;
    sqlx::query("UPDATE printer_jobs SET print_id = ?1 WHERE id = ?2").bind(id).bind(job_id).execute(&pool).await.unwrap();

    let (status, body) = send(&pool, "DELETE", &format!("/api/prints/{id}"), None).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let count = |sql: &'static str| { let pool = pool.clone(); async move { sqlx::query_scalar::<_, i64>(sql).fetch_one(&pool).await.unwrap() } };
    assert_eq!(count("SELECT COUNT(*) FROM prints").await, 0);
    assert_eq!(count("SELECT COUNT(*) FROM print_failure_modes").await, 0);
    assert_eq!(count("SELECT COUNT(*) FROM print_hourly_weather").await, 0);
    let job_print: Option<i64> = sqlx::query_scalar("SELECT print_id FROM printer_jobs WHERE id = ?1").bind(job_id).fetch_one(&pool).await.unwrap();
    assert_eq!(job_print, None, "the Job is kept, just unlinked");
}

async fn print_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM prints").fetch_one(pool).await.unwrap()
}


fn outcome(status: &str, failure_modes: Vec<&str>) -> Value {
    json!({ "status": status, "failureModes": failure_modes, "notes": "stringy on the overhangs", "actualRoomTempC": 19.5, "cleanBuildPlate": false })
}

// spec: 003/FR-010
#[tokio::test]
async fn prints_cannot_be_created_by_hand() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;

    let (status, _) = send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;

    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(print_count(&pool).await, 0);
}

// spec: 003/FR-030
#[tokio::test]
async fn manual_job_matching_endpoints_are_gone() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let id = created["print"]["id"].as_i64().unwrap();

    // "job-match" now falls through to /api/prints/{id} and fails to parse as an id.
    let (status, _) = send_raw(&pool, "GET", &format!("/api/prints/job-match?printerId={}&startedAt=2026-08-15T10:00:00Z", f.printer_id)).await;
    assert!(status.is_client_error(), "{status}");
    let (status, _) = send(&pool, "POST", &format!("/api/prints/{id}/attach-job"), Some(json!({ "jobId": 1 }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// spec: 003/FR-011, 003/FR-013
#[tokio::test]
async fn annotating_sets_only_the_outcome_fields() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = common::create_print(&pool, Some(print_body(&f, "Success", vec![]))).await;
    let id = created["print"]["id"].as_i64().unwrap();
    let other_printer = seed_printer(&pool, "Other").await;
    let mut body = outcome("Partial", vec!["Stringing", "Stringing", "Warping"]);
    // Anything else in the request is ignored, never applied.
    body["printerId"] = json!(other_printer);
    body["startedAt"] = json!("2020-01-01T00:00:00Z");

    let (status, res) = send(&pool, "PUT", &format!("/api/prints/{id}"), Some(body)).await;

    assert_eq!(status, StatusCode::OK, "{res:?}");
    let p = &res["print"];
    assert_eq!(p["status"], "Partial");
    assert_eq!(p["notes"], "stringy on the overhangs");
    assert_eq!(p["actualRoomTempC"], 19.5);
    assert_eq!(p["cleanBuildPlate"], false);
    let modes: Vec<&str> = p["failureModes"].as_array().unwrap().iter().map(|m| m["mode"].as_str().unwrap()).collect();
    assert_eq!(modes.len(), 2, "de-duplicated: {modes:?}");
    assert_eq!(p["printerId"], f.printer_id, "printer unchanged");
    assert_eq!(p["startedAt"], "2026-08-15T10:00:00Z", "times unchanged");
    assert_eq!(p["profileId"], f.profile_id);
}

// spec: 003/FR-011, 003/FR-012, 003/FR-013
#[tokio::test]
async fn annotating_rejects_bad_outcomes_and_unfinished_prints() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, done) = common::create_print(&pool, Some(print_body(&f, "Failed", vec!["Clog"]))).await;
    let done = done["print"]["id"].as_i64().unwrap();
    let mut running = print_body(&f, "InProgress", vec![]);
    running["input"]["endedAt"] = json!("");
    let (_, running) = common::create_print(&pool, Some(running)).await;
    let running = running["print"]["id"].as_i64().unwrap();

    for (id, body, expected) in [
        (done, outcome("InProgress", vec![]), "Unknown status."),
        (done, outcome("Great", vec![]), "Unknown status."),
        (done, outcome("Failed", vec!["Spaghetti"]), "Unknown failure mode: Spaghetti."),
        (done, outcome("Success", vec!["Warping"]), "failure_modes_require_failed_or_partial"),
        (running, outcome("Failed", vec![]), "still_in_progress"),
    ] {
        let (status, res) = send(&pool, "PUT", &format!("/api/prints/{id}"), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{expected}: {res:?}");
        assert_eq!(res["error"], expected);
    }
    let (status, _) = send(&pool, "PUT", "/api/prints/999", Some(outcome("Failed", vec![]))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, unchanged) = send(&pool, "GET", &format!("/api/prints/{done}"), None).await;
    assert_eq!(unchanged["status"], "Failed");
    assert_eq!(unchanged["notes"], "test print");
}

// Status only — for responses that may not be JSON.
async fn send_raw(pool: &sqlx::SqlitePool, method: &str, uri: &str) -> (StatusCode, Vec<u8>) {
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(Request::builder().method(method).uri(uri).header("cookie", common::auth_cookie_header(pool).await).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    (status, axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec())
}
