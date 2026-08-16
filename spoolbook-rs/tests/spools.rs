use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
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
    let (_, body) = send(
        pool,
        "POST",
        "/api/filaments",
        Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" })),
    )
    .await;
    body["entry"]["id"].as_i64().unwrap()
}

#[tokio::test]
async fn create_persists_and_returns_the_spool_with_its_filament() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;

    let (status, body) = send(
        &pool,
        "POST",
        "/api/spools",
        Some(json!({
            "filamentId": filament_id, "lotCode": "LOT-1", "purchasedAt": "2026-08-01",
            "openedAt": null, "emptiedAt": null, "weightGrams": 1000, "diameterMm": 1.75, "notes": null
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["spool"]["filamentId"], filament_id);
    assert_eq!(body["spool"]["lotCode"], "LOT-1");
    assert_eq!(body["spool"]["weightGrams"], 1000);
    assert_eq!(body["spool"]["diameterMm"], 1.75);
    assert_eq!(body["spool"]["filament"]["brand"], "Bambu Lab");
}

#[tokio::test]
async fn list_all_includes_the_nested_filament() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;

    let (status, body) = send(&pool, "GET", "/api/spools", None).await;

    assert_eq!(status, StatusCode::OK);
    let entries = body.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["filament"]["material"], "PLA");
}

#[tokio::test]
async fn get_by_id_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "GET", "/api/spools/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_persists_changes() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": "OLD", "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    let id = created["spool"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "PUT", &format!("/api/spools/{id}"), Some(json!({
        "lotCode": "NEW", "purchasedAt": null, "openedAt": null, "emptiedAt": null,
        "weightGrams": null, "diameterMm": null, "notes": "rewound"
    }))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["spool"]["lotCode"], "NEW");
    assert_eq!(body["spool"]["notes"], "rewound");
}

async fn seed_profile(pool: &sqlx::SqlitePool, filament_id: i64, spool_id: i64) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO print_profiles (filament_id, spool_id, name, nozzle_temp_c) VALUES (?1, ?2, 'p', 220) RETURNING id",
    )
    .bind(filament_id)
    .bind(spool_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_printer(pool: &sqlx::SqlitePool) -> i64 {
    let (_, body) = send(pool, "POST", "/api/printers", Some(json!({
        "name": "P2S #1", "model": "P2S", "ipAddress": null, "accessCode": null, "serialNumber": null
    }))).await;
    body["printer"]["id"].as_i64().unwrap()
}

#[tokio::test]
async fn delete_rejects_a_spool_with_profiles() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    let id = created["spool"]["id"].as_i64().unwrap();
    seed_profile(&pool, filament_id, id).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/spools/{id}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "has_profiles");

    let (_, all) = send(&pool, "GET", "/api/spools", None).await;
    assert_eq!(all.as_array().unwrap().len(), 1, "spool must survive the rejected delete");
}

#[tokio::test]
async fn delete_rejects_a_spool_with_prints() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    let id = created["spool"]["id"].as_i64().unwrap();
    let profile_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, 'p', 220) RETURNING id",
    )
    .bind(filament_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let printer_id = seed_printer(&pool).await;
    send(&pool, "POST", "/api/prints", Some(json!({
        "profileId": profile_id, "spoolId": id, "printerId": printer_id,
        "input": {
            "startedAt": "2026-08-15T10:00:00Z", "endedAt": "2026-08-15T12:00:00Z",
            "status": "Success", "notes": null, "amsHumidityPct": null,
            "actualRoomTempC": null, "cleanBuildPlate": true,
            "projectId": null, "projectPlaterId": null,
            "failureModes": []
        }
    }))).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/spools/{id}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "has_prints");
}

#[tokio::test]
async fn delete_removes_the_spool() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    let id = created["spool"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "DELETE", &format!("/api/spools/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    let (_, all) = send(&pool, "GET", "/api/spools", None).await;
    assert_eq!(all.as_array().unwrap().len(), 0);
}
