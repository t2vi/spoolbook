use axum::body::Body;
use axum::http::{Request, StatusCode};
use rumqttc::{AsyncClient, MqttOptions};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn send(pool: &sqlx::SqlitePool, store: &spoolbook_rs::printer_mqtt::LiveStatusStore, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let response = spoolbook_rs::app_with_live_status(pool.clone(), store.clone())
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

async fn seed_printer(pool: &sqlx::SqlitePool, serial_number: Option<&str>) -> i64 {
    sqlx::query_scalar::<_, i64>("INSERT INTO printers (name, serial_number) VALUES ('Garage P2S', ?1) RETURNING id")
        .bind(serial_number)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn control_returns_not_found_for_missing_printer() {
    let pool = test_pool().await;
    let store = spoolbook_rs::printer_mqtt::new_store();

    let (status, body) = send(&pool, &store, "POST", "/api/printers/999/control", Some(json!({ "command": "pause" }))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[tokio::test]
async fn control_returns_not_found_for_a_printer_with_no_serial_number() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool, None).await;
    let store = spoolbook_rs::printer_mqtt::new_store();

    let (status, body) = send(&pool, &store, "POST", &format!("/api/printers/{printer_id}/control"), Some(json!({ "command": "pause" }))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[tokio::test]
async fn control_rejects_an_unknown_command() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool, Some("SERIAL1")).await;
    let store = spoolbook_rs::printer_mqtt::new_store();

    let (status, body) = send(&pool, &store, "POST", &format!("/api/printers/{printer_id}/control"), Some(json!({ "command": "reboot" }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"], "Unknown command.");
}

#[tokio::test]
async fn control_fails_when_the_printer_has_no_live_connection() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool, Some("SERIAL1")).await;
    let store = spoolbook_rs::printer_mqtt::new_store();

    let (status, body) = send(&pool, &store, "POST", &format!("/api/printers/{printer_id}/control"), Some(json!({ "command": "pause" }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"], "Printer isn't connected — telemetry link is down or still reconnecting.");
}

#[tokio::test]
async fn control_publishes_when_a_live_client_is_registered() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool, Some("SERIAL1")).await;
    let store = spoolbook_rs::printer_mqtt::new_store();

    // A real (but unpolled/unconnected) client — proves publish_command's plumbing dispatches
    // correctly without needing a live broker; the actual network handshake is unverified,
    // same boundary as the rest of this MQTT tier.
    let (client, _eventloop) = AsyncClient::new(MqttOptions::new("test-client", "127.0.0.1", 8883), 10);
    store.write().await.entry(printer_id).or_default().client = Some(client);

    for command in ["pause", "resume", "stop"] {
        let (status, body) = send(&pool, &store, "POST", &format!("/api/printers/{printer_id}/control"), Some(json!({ "command": command }))).await;
        assert_eq!(status, StatusCode::OK, "{command}: {body:?}");
        assert_eq!(body["ok"], true, "{command}");
    }
}
