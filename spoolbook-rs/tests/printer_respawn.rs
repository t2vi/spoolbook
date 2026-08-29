// A printer configured while the process is already running (the normal case under Docker: fresh
// data volume, printer added through the UI, container never bounced) must get its live-telemetry
// loop started right then -- not only at the next startup's spawn_all. Without it, Test Connection
// works but the card stays "Not connected" and Print fails with "Printer isn't connected".
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

async fn send(
    pool: &sqlx::SqlitePool,
    supervisor: &spoolbook_rs::printer_mqtt::ConnSupervisor,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let app = spoolbook_rs::app_with_camera_supervised(
        pool.clone(),
        spoolbook_rs::printer_mqtt::new_store(),
        spoolbook_rs::printer_camera::new_registry(),
        supervisor.clone(),
    );
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let response = app
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

#[tokio::test]
async fn create_then_delete_starts_and_stops_the_telemetry_loop() {
    let pool = test_pool().await;
    let supervisor = spoolbook_rs::printer_mqtt::new_supervisor();

    let body = json!({ "name": "P2S", "model": "P2S", "ipAddress": "192.168.1.50", "accessCode": "12345678", "serialNumber": "ABC123" });
    let (status, created) = send(&pool, &supervisor, "POST", "/api/printers", Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    let id = created["printer"]["id"].as_i64().unwrap();

    assert!(
        supervisor.lock().await.contains_key(&id),
        "creating a printer with connection details should start its telemetry loop"
    );

    let (status, _) = send(&pool, &supervisor, "DELETE", &format!("/api/printers/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !supervisor.lock().await.contains_key(&id),
        "deleting a printer should stop its telemetry loop"
    );
}

#[tokio::test]
async fn create_without_connection_details_registers_no_loop() {
    let pool = test_pool().await;
    let supervisor = spoolbook_rs::printer_mqtt::new_supervisor();

    let body = json!({ "name": "bare", "model": "P2S" });
    let (status, created) = send(&pool, &supervisor, "POST", "/api/printers", Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    let id = created["printer"]["id"].as_i64().unwrap();

    assert!(!supervisor.lock().await.contains_key(&id));
}
