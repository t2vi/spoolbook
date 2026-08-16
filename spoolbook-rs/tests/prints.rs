#![recursion_limit = "512"]
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
            "status": status, "notes": "test print", "amsHumidityPct": 30,
            "actualRoomTempC": 22.5, "cleanBuildPlate": true,
            "projectId": null, "projectPlaterId": null,
            "failureModes": failure_modes
        }
    })
}

#[tokio::test]
async fn create_persists_and_nests_profile_spool_printer() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;

    let (status, body) = send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let p = &body["print"];
    assert_eq!(p["profileId"], f.profile_id);
    assert_eq!(p["spoolId"], f.spool_id);
    assert_eq!(p["printerId"], f.printer_id);
    assert_eq!(p["profile"]["name"], "Default");
    assert_eq!(p["spool"]["filament"]["brand"], "Bambu Lab");
    assert_eq!(p["printer"]["name"], "P2S #1");
    assert_eq!(p["status"], "Success");
    assert_eq!(p["notes"], "test print");
    assert_eq!(p["failureModes"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn create_persists_failure_modes_when_not_success() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;

    let (status, body) = send(&pool, "POST", "/api/prints", Some(print_body(&f, "Failed", vec!["Warping", "Stringing"]))).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let modes: Vec<&str> = body["print"]["failureModes"].as_array().unwrap().iter().map(|m| m["mode"].as_str().unwrap()).collect();
    assert_eq!(modes.len(), 2);
    assert!(modes.contains(&"Warping"));
    assert!(modes.contains(&"Stringing"));
}

#[tokio::test]
async fn create_rejects_failure_modes_on_a_success_status() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;

    let (status, body) = send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec!["Warping"]))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "failure_modes_require_failed_or_partial");
}

#[tokio::test]
async fn create_nests_project_when_provided() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let project_id = seed_project(&pool).await;
    let mut body = print_body(&f, "Success", vec![]);
    body["input"]["projectId"] = json!(project_id);
    body["input"]["projectPlaterId"] = json!("1");

    let (status, resp) = send(&pool, "POST", "/api/prints", Some(body)).await;

    assert_eq!(status, StatusCode::OK, "{resp:?}");
    assert_eq!(resp["print"]["project"]["fileName"], "x.3mf");
    assert_eq!(resp["print"]["projectPlaterId"], "1");
}

#[tokio::test]
async fn get_by_id_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "GET", "/api/prints/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_by_id_returns_the_full_nested_print() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;
    let id = created["print"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "GET", &format!("/api/prints/{id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], id);
    assert_eq!(body["profile"]["nozzleTempC"], 220);
}

#[tokio::test]
async fn list_filters_by_printer_and_caps_at_five() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    for _ in 0..6 {
        send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;
    }
    let other_printer_id = seed_printer(&pool, "P2S #2").await;
    let cross_printer = Fixtures { filament_id: f.filament_id, profile_id: f.profile_id, spool_id: f.spool_id, printer_id: other_printer_id };
    send(&pool, "POST", "/api/prints", Some(print_body(&cross_printer, "Success", vec![]))).await;

    let (status, body) = send(&pool, "GET", &format!("/api/prints?printerId={}", f.printer_id), None).await;

    assert_eq!(status, StatusCode::OK);
    let prints = body.as_array().unwrap();
    assert_eq!(prints.len(), 5, "capped at 5 even though 6 were created for this printer");
    assert!(prints.iter().all(|p| p["printerId"] == f.printer_id), "excludes the other printer's print");
}

#[tokio::test]
async fn list_without_printer_id_returns_everything() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;

    let (status, body) = send(&pool, "GET", "/api/prints", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn inventory_filters_by_status_and_paginates() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;
    send(&pool, "POST", "/api/prints", Some(print_body(&f, "Failed", vec!["Clog"]))).await;

    let (status, body) = send(&pool, "GET", "/api/prints/inventory?status=Success&page=1&pageSize=20", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["prints"][0]["status"], "Success");
}

#[tokio::test]
async fn recommend_profile_picks_success_over_failed() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let project_id = seed_project(&pool).await;

    let mut failed_body = print_body(&f, "Failed", vec!["Warping"]);
    failed_body["input"]["projectId"] = json!(project_id);
    send(&pool, "POST", "/api/prints", Some(failed_body)).await;

    let success_profile_id = seed_profile(&pool, f.filament_id, "Success Profile").await;
    let success_f = Fixtures { filament_id: f.filament_id, profile_id: success_profile_id, spool_id: f.spool_id, printer_id: f.printer_id };
    let mut success_body = print_body(&success_f, "Success", vec![]);
    success_body["input"]["projectId"] = json!(project_id);
    send(&pool, "POST", "/api/prints", Some(success_body)).await;

    let (status, body) = send(&pool, "GET", &format!("/api/prints/recommend-profile?projectId={project_id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Success Profile");
}

#[tokio::test]
async fn recommend_profile_returns_null_when_no_prints_reference_the_project() {
    let pool = test_pool().await;
    let project_id = seed_project(&pool).await;
    let (status, body) = send(&pool, "GET", &format!("/api/prints/recommend-profile?projectId={project_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Null);
}

#[tokio::test]
async fn update_persists_changes() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;
    let id = created["print"]["id"].as_i64().unwrap();

    let update = json!({
        "printerId": f.printer_id,
        "input": {
            "startedAt": "2026-08-15T10:00:00Z", "endedAt": "2026-08-15T12:00:00Z",
            "status": "Partial", "notes": "updated notes", "amsHumidityPct": 35,
            "actualRoomTempC": 23.0, "cleanBuildPlate": false,
            "projectId": null, "projectPlaterId": null,
            "failureModes": ["Clog"]
        }
    });
    let (status, body) = send(&pool, "PUT", &format!("/api/prints/{id}"), Some(update)).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["print"]["status"], "Partial");
    assert_eq!(body["print"]["notes"], "updated notes");
    let modes: Vec<&str> = body["print"]["failureModes"].as_array().unwrap().iter().map(|m| m["mode"].as_str().unwrap()).collect();
    assert_eq!(modes, vec!["Clog"]);
}

#[tokio::test]
async fn update_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let update = json!({
        "printerId": f.printer_id,
        "input": { "startedAt": "2026-08-15T10:00:00Z", "endedAt": "2026-08-15T12:00:00Z", "status": "Success", "notes": null, "amsHumidityPct": null, "actualRoomTempC": null, "cleanBuildPlate": null, "projectId": null, "projectPlaterId": null, "failureModes": [] }
    });
    let (status, _) = send(&pool, "PUT", "/api/prints/999", Some(update)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_removes_the_print() {
    let pool = test_pool().await;
    let f = seed_all(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/prints", Some(print_body(&f, "Success", vec![]))).await;
    let id = created["print"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "DELETE", &format!("/api/prints/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    let (_, all) = send(&pool, "GET", "/api/prints", None).await;
    assert_eq!(all.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn delete_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "DELETE", "/api/prints/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
