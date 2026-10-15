mod common;
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
    let (_, body) = send(
        pool,
        "POST",
        "/api/filaments",
        Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" })),
    )
    .await;
    body["entry"]["id"].as_i64().unwrap()
}

// spec: 007/FR-030
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

// spec: 007/FR-031
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

// spec: 007/FR-031
#[tokio::test]
async fn get_by_id_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "GET", "/api/spools/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// spec: 007/FR-032
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

// spec: 007/FR-033
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

// spec: 007/FR-033
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
            "status": "Success", "notes": null, "cleanBuildPlate": true,
            "projectId": null, "projectPlaterId": null,
            "failureModes": []
        }
    }))).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/spools/{id}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "has_prints");
}

// spec: 007/FR-033
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

fn spool_body(filament_id: i64, purchased: Option<&str>, opened: Option<&str>, emptied: Option<&str>) -> Value {
    json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": purchased, "openedAt": opened, "emptiedAt": emptied,
        "weightGrams": null, "diameterMm": null, "notes": null
    })
}

async fn spool_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM spools").fetch_one(pool).await.unwrap()
}

// spec: 007/FR-030
#[tokio::test]
async fn create_rejects_an_unknown_filament() {
    let pool = test_pool().await;

    let (status, body) = send(&pool, "POST", "/api/spools", Some(spool_body(999, None, None, None))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Unknown filament.");
    assert_eq!(spool_count(&pool).await, 0);
}

// spec: 007/FR-030
#[tokio::test]
async fn create_rejects_dates_out_of_order() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let cases = [
        (Some("2026-08-10"), Some("2026-08-01"), None),             // opened before purchased
        (None, Some("2026-08-10"), Some("2026-08-01")),             // emptied before opened
        (Some("2026-08-10"), None, Some("2026-08-01")),             // emptied before purchased
        (Some("2026-08-10T09:00:00Z"), Some("2026-08-01"), None),   // mixed formats compare by day
    ];

    for (purchased, opened, emptied) in cases {
        let (status, body) = send(&pool, "POST", "/api/spools", Some(spool_body(filament_id, purchased, opened, emptied))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{purchased:?} {opened:?} {emptied:?}");
        assert_eq!(body["error"], "Dates are out of order — purchased, then opened, then emptied.");
    }
    assert_eq!(spool_count(&pool).await, 0);
}

// spec: 007/FR-030
#[tokio::test]
async fn create_accepts_same_day_and_partial_dates() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;

    for (purchased, opened, emptied) in [(Some("2026-08-01"), Some("2026-08-01T20:00:00Z"), Some("2026-08-01")), (None, None, Some("2026-08-01"))] {
        let (status, body) = send(&pool, "POST", "/api/spools", Some(spool_body(filament_id, purchased, opened, emptied))).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
}

// spec: 007/FR-032
#[tokio::test]
async fn update_rejects_dates_out_of_order() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", "/api/spools", Some(spool_body(filament_id, Some("2026-08-01"), None, None))).await;
    let id = created["spool"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "PUT", &format!("/api/spools/{id}"), Some(spool_body(filament_id, Some("2026-08-10"), Some("2026-08-01"), None))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Dates are out of order — purchased, then opened, then emptied.");
    let purchased: Option<String> = sqlx::query_scalar("SELECT purchased_at FROM spools WHERE id = ?1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(purchased.as_deref(), Some("2026-08-01"), "unchanged");
}
