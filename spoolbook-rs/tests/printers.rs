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

fn printer_body(name: &str) -> Value {
    json!({ "name": name, "model": "P2S", "ipAddress": "192.168.1.50", "accessCode": "12345678", "serialNumber": "ABC123" })
}

#[tokio::test]
async fn list_returns_empty_array_when_none_exist() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "GET", "/api/printers", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn create_persists_and_returns_the_printer() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "POST", "/api/printers", Some(printer_body("P2S #1"))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["printer"]["name"], "P2S #1");
    assert_eq!(body["printer"]["model"], "P2S");
    assert_eq!(body["printer"]["ipAddress"], "192.168.1.50");
    assert_eq!(body["printer"]["accessCode"], "12345678");
    assert_eq!(body["printer"]["serialNumber"], "ABC123");
}

#[tokio::test]
async fn create_rejects_blank_name() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "POST", "/api/printers", Some(printer_body(""))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn create_rejects_duplicate_name() {
    let pool = test_pool().await;
    send(&pool, "POST", "/api/printers", Some(printer_body("P2S #1"))).await;
    let (status, body) = send(&pool, "POST", "/api/printers", Some(printer_body("P2S #1"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "duplicate");
}

#[tokio::test]
async fn update_persists_changes() {
    let pool = test_pool().await;
    let (_, created) = send(&pool, "POST", "/api/printers", Some(printer_body("P2S #1"))).await;
    let id = created["printer"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "PUT", &format!("/api/printers/{id}"), Some(printer_body("P2S renamed"))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["printer"]["name"], "P2S renamed");
}

#[tokio::test]
async fn update_rejects_duplicate_against_another_row() {
    let pool = test_pool().await;
    send(&pool, "POST", "/api/printers", Some(printer_body("P2S #1"))).await;
    let (_, second) = send(&pool, "POST", "/api/printers", Some(printer_body("P2S #2"))).await;
    let second_id = second["printer"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "PUT", &format!("/api/printers/{second_id}"), Some(printer_body("P2S #1"))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "duplicate");
}

#[tokio::test]
async fn update_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "PUT", "/api/printers/999", Some(printer_body("Ghost"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_rejects_a_printer_with_prints() {
    let pool = test_pool().await;
    let (_, created) = send(&pool, "POST", "/api/printers", Some(printer_body("P2S #1"))).await;
    let printer_id = created["printer"]["id"].as_i64().unwrap();

    let (_, filament) = send(&pool, "POST", "/api/filaments", Some(json!({
        "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black"
    }))).await;
    let filament_id = filament["entry"]["id"].as_i64().unwrap();
    let (_, spool) = send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    let spool_id = spool["spool"]["id"].as_i64().unwrap();
    let profile_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, 'p', 220) RETURNING id",
    )
    .bind(filament_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    send(&pool, "POST", "/api/prints", Some(json!({
        "profileId": profile_id, "spoolId": spool_id, "printerId": printer_id,
        "input": {
            "startedAt": "2026-08-15T10:00:00Z", "endedAt": "2026-08-15T12:00:00Z",
            "status": "Success", "notes": null, "amsHumidityPct": null,
            "actualRoomTempC": null, "cleanBuildPlate": true,
            "projectId": null, "projectPlaterId": null,
            "failureModes": []
        }
    }))).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/printers/{printer_id}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "has_prints");

    let (_, all) = send(&pool, "GET", "/api/printers", None).await;
    assert_eq!(all.as_array().unwrap().len(), 1, "printer must survive the rejected delete");
}

#[tokio::test]
async fn delete_removes_the_printer() {
    let pool = test_pool().await;
    let (_, created) = send(&pool, "POST", "/api/printers", Some(printer_body("P2S #1"))).await;
    let id = created["printer"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "DELETE", &format!("/api/printers/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    let (_, all) = send(&pool, "GET", "/api/printers", None).await;
    assert_eq!(all.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn delete_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "DELETE", "/api/printers/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
