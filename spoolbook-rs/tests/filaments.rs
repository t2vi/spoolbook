use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

async fn post_json(pool: &sqlx::SqlitePool, uri: &str, body: Value) -> (StatusCode, Value) {
    send(pool, "POST", uri, Some(body)).await
}

// A pool with more than 1 connection would give each connection its own separate
// in-memory database — max_connections(1) keeps every query on the same one.
async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to open in-memory db");
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

#[tokio::test]
async fn list_all_returns_empty_array_when_no_filaments() {
    let pool = test_pool().await;
    let app = spoolbook_rs::app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/filaments/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body.as_ref(), b"[]");
}

#[tokio::test]
async fn create_persists_and_returns_the_entry() {
    let pool = test_pool().await;
    let input = json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" });

    let (status, body) = post_json(&pool, "/api/filaments", input).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["entry"]["brand"], "Bambu Lab");
    assert_eq!(body["entry"]["material"], "PLA");
    assert_eq!(body["entry"]["variant"], "Basic");
    assert_eq!(body["entry"]["color"], "Black");
    assert!(body["entry"]["id"].is_i64());
}

#[tokio::test]
async fn create_rejects_exact_duplicate() {
    let pool = test_pool().await;
    let input = json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" });

    let (first_status, _) = post_json(&pool, "/api/filaments", input.clone()).await;
    assert_eq!(first_status, StatusCode::OK);

    let (second_status, second_body) = post_json(&pool, "/api/filaments", input).await;

    assert_eq!(second_status, StatusCode::BAD_REQUEST);
    assert_eq!(second_body["ok"], false);
    assert_eq!(second_body["error"], "duplicate");
}

#[tokio::test]
async fn update_persists_changes_and_returns_the_entry() {
    let pool = test_pool().await;
    let (_, created) = post_json(
        &pool,
        "/api/filaments",
        json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }),
    )
    .await;
    let id = created["entry"]["id"].as_i64().unwrap();

    let (status, body) = send(
        &pool,
        "PUT",
        &format!("/api/filaments/{id}"),
        Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "White" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["entry"]["color"], "White");
}

#[tokio::test]
async fn update_returns_not_found_for_missing_id() {
    let pool = test_pool().await;

    let (status, body) = send(
        &pool,
        "PUT",
        "/api/filaments/999",
        Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn update_rejects_duplicate_against_a_different_row() {
    let pool = test_pool().await;
    post_json(
        &pool,
        "/api/filaments",
        json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }),
    )
    .await;
    let (_, second) = post_json(
        &pool,
        "/api/filaments",
        json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "White" }),
    )
    .await;
    let second_id = second["entry"]["id"].as_i64().unwrap();

    // Renaming the second entry's color to match the first is a real duplicate.
    let (status, body) = send(
        &pool,
        "PUT",
        &format!("/api/filaments/{second_id}"),
        Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "duplicate");
}

#[tokio::test]
async fn update_allows_saving_a_row_unchanged() {
    let pool = test_pool().await;
    let (_, created) = post_json(
        &pool,
        "/api/filaments",
        json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }),
    )
    .await;
    let id = created["entry"]["id"].as_i64().unwrap();

    // Same values back — must not trip the duplicate check against itself.
    let (status, _) = send(
        &pool,
        "PUT",
        &format!("/api/filaments/{id}"),
        Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn delete_removes_the_entry() {
    let pool = test_pool().await;
    let (_, created) = post_json(
        &pool,
        "/api/filaments",
        json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }),
    )
    .await;
    let id = created["entry"]["id"].as_i64().unwrap();

    let (status, body) = send(&pool, "DELETE", &format!("/api/filaments/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    let (_, all) = send(&pool, "GET", "/api/filaments/all", None).await;
    assert_eq!(all.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn delete_rejects_a_filament_with_spools() {
    let pool = test_pool().await;
    let (_, created) = post_json(
        &pool,
        "/api/filaments",
        json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" }),
    )
    .await;
    let id = created["entry"]["id"].as_i64().unwrap();
    send(&pool, "POST", "/api/spools", Some(json!({
        "filamentId": id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/filaments/{id}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "has_spools");

    let (_, all) = send(&pool, "GET", "/api/filaments/all", None).await;
    assert_eq!(all.as_array().unwrap().len(), 1, "filament must survive the rejected delete");
}

#[tokio::test]
async fn delete_returns_not_found_for_missing_id() {
    let pool = test_pool().await;

    let (status, body) = send(&pool, "DELETE", "/api/filaments/999", None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn search_returns_pagination_metadata() {
    let pool = test_pool().await;
    post_json(&pool, "/api/filaments", json!({ "brand": "Bambu Lab", "material": "PLA", "variant": null, "color": "Black" })).await;
    post_json(&pool, "/api/filaments", json!({ "brand": "Bambu Lab", "material": "PETG", "variant": null, "color": "White" })).await;
    post_json(&pool, "/api/filaments", json!({ "brand": "Polymaker", "material": "PLA", "variant": null, "color": "Red" })).await;

    let (status, body) = send(&pool, "GET", "/api/filaments?brand=&material=&page=1&pageSize=20", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    assert_eq!(body["page"], 1);
    assert_eq!(body["pageSize"], 20);
    assert_eq!(body["totalPages"], 1);
    assert_eq!(body["entries"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn search_filters_by_brand() {
    let pool = test_pool().await;
    post_json(&pool, "/api/filaments", json!({ "brand": "Bambu Lab", "material": "PLA", "variant": null, "color": "Black" })).await;
    post_json(&pool, "/api/filaments", json!({ "brand": "Polymaker", "material": "PLA", "variant": null, "color": "Red" })).await;

    let (_, body) = send(&pool, "GET", "/api/filaments?brand=Polymaker&material=&page=1&pageSize=20", None).await;

    assert_eq!(body["total"], 1);
    assert_eq!(body["entries"][0]["brand"], "Polymaker");
}

#[tokio::test]
async fn search_paginates() {
    let pool = test_pool().await;
    for color in ["Black", "White", "Red"] {
        post_json(&pool, "/api/filaments", json!({ "brand": "Bambu Lab", "material": "PLA", "variant": color, "color": color })).await;
    }

    let (_, body) = send(&pool, "GET", "/api/filaments?brand=&material=&page=1&pageSize=2", None).await;

    assert_eq!(body["total"], 3);
    assert_eq!(body["totalPages"], 2);
    assert_eq!(body["entries"].as_array().unwrap().len(), 2);
}
