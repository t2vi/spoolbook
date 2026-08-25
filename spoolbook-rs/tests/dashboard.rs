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

#[tokio::test]
async fn returns_zeroed_metrics_and_all_four_print_statuses_when_empty() {
    let pool = test_pool().await;

    let (status, body) = send(&pool, "GET", "/api/dashboard", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["metrics"]["filamentCount"], 0);
    assert_eq!(body["metrics"]["lastFilamentSyncAt"], Value::Null);
    assert_eq!(body["metrics"]["filamentsByBrand"].as_array().unwrap().len(), 0);
    assert_eq!(body["metrics"]["spoolsByStatus"], json!([
        { "label": "Unopened", "count": 0 },
        { "label": "Opened", "count": 0 },
        { "label": "Empty", "count": 0 }
    ]));
    assert_eq!(body["metrics"]["printsByStatus"], json!([
        { "label": "Success", "count": 0 },
        { "label": "Failed", "count": 0 },
        { "label": "Partial", "count": 0 },
        { "label": "InProgress", "count": 0 }
    ]));
    assert_eq!(body["profileCount"], 0);
}

#[tokio::test]
async fn counts_filaments_by_brand_and_material_descending() {
    let pool = test_pool().await;
    send(&pool, "POST", "/api/filaments", Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }))).await;
    send(&pool, "POST", "/api/filaments", Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Matte", "color": "White" }))).await;
    send(&pool, "POST", "/api/filaments", Some(json!({ "brand": "Polymaker", "material": "PETG", "variant": null, "color": "Clear" }))).await;

    let (_, body) = send(&pool, "GET", "/api/dashboard", None).await;

    assert_eq!(body["metrics"]["filamentCount"], 3);
    assert_eq!(body["metrics"]["filamentsByBrand"][0], json!({ "label": "Bambu Lab", "count": 2 }));
    assert_eq!(body["metrics"]["filamentsByMaterial"][0], json!({ "label": "PLA", "count": 2 }));
}

#[tokio::test]
async fn buckets_spools_by_opened_and_emptied_state() {
    let pool = test_pool().await;
    let (_, filament) = send(&pool, "POST", "/api/filaments", Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }))).await;
    let filament_id = filament["entry"]["id"].as_i64().unwrap();

    send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": "2026-08-01", "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": "2026-08-01", "emptiedAt": "2026-08-10", "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;

    let (_, body) = send(&pool, "GET", "/api/dashboard", None).await;

    assert_eq!(body["metrics"]["spoolsByStatus"], json!([
        { "label": "Unopened", "count": 1 },
        { "label": "Opened", "count": 1 },
        { "label": "Empty", "count": 1 }
    ]));
}

#[tokio::test]
async fn reports_last_filament_sync_at_and_profile_count_from_settings_and_profiles() {
    let pool = test_pool().await;
    send(&pool, "POST", "/api/settings", Some(json!({ "additionalFilamentSourceUrls": null }))).await;
    sqlx::query("UPDATE app_settings SET last_filament_sync_at = '2026-08-16T00:00:00Z' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let (_, filament) = send(&pool, "POST", "/api/filaments", Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }))).await;
    let filament_id = filament["entry"]["id"].as_i64().unwrap();
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, 'p', 220) RETURNING id",
    )
    .bind(filament_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (_, body) = send(&pool, "GET", "/api/dashboard", None).await;

    assert_eq!(body["metrics"]["lastFilamentSyncAt"], "2026-08-16T00:00:00Z");
    assert_eq!(body["profileCount"], 1);
}
