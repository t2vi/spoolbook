mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
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

// spec: 007/FR-012
#[tokio::test]
async fn list_colors_returns_empty_array_when_none_seeded() {
    let pool = test_pool().await;
    let cookie = common::auth_cookie_header(&pool).await;
    let app = spoolbook_rs::app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/filament-colors")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body.as_ref(), b"[]");
}

// spec: 007/FR-012
#[tokio::test]
async fn list_colors_returns_seeded_rows_sorted_by_name() {
    let pool = test_pool().await;
    sqlx::query("INSERT INTO filament_colors (name, hex) VALUES ('White', '#FFFFFF'), ('Black', '#000000')")
        .execute(&pool)
        .await
        .unwrap();

    let cookie = common::auth_cookie_header(&pool).await;
    let app = spoolbook_rs::app(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/filament-colors")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let names: Vec<&str> = json.as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Black", "White"]);
}

async fn send(pool: &sqlx::SqlitePool, method: &str, uri: &str, body: Option<serde_json::Value>) -> (StatusCode, serde_json::Value) {
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", common::auth_cookie_header(pool).await)
                .body(body.map(|b| Body::from(b.to_string())).unwrap_or_default())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() })
}

async fn add_color(pool: &sqlx::SqlitePool, name: &str, hex: &str) -> i64 {
    let (status, body) = send(pool, "POST", "/api/filament-colors", Some(serde_json::json!({ "name": name, "hex": hex }))).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    body["color"]["id"].as_i64().unwrap()
}

async fn add_filament(pool: &sqlx::SqlitePool, color: &str) -> i64 {
    let (_, body) = send(pool, "POST", "/api/filaments", Some(serde_json::json!({ "brand": "Bambu Lab", "material": "PLA", "variant": null, "color": color }))).await;
    body["entry"]["id"].as_i64().unwrap()
}

// spec: 007/FR-014, 007/FR-015
#[tokio::test]
async fn create_validates_name_hex_and_duplicates() {
    let pool = test_pool().await;
    add_color(&pool, "Black", "#000000").await;

    for (name, hex, expected) in [
        ("  ", "#000000", "Name is required"),
        ("Red", "red", "invalid_hex"),
        ("Red", "#12345", "invalid_hex"),
        ("Red", "#1A1A1A,#GGGGGG", "invalid_hex"),
        ("Red", " , ", "invalid_hex"),
        ("Black", "#111111", "duplicate"),
    ] {
        let (status, body) = send(&pool, "POST", "/api/filament-colors", Some(serde_json::json!({ "name": name, "hex": hex }))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name:?} {hex:?}: {body:?}");
        assert_eq!(body["error"], expected, "{name:?} {hex:?}");
    }

    let (status, body) = send(&pool, "POST", "/api/filament-colors", Some(serde_json::json!({ "name": " Black + Gold ", "hex": "#1a1a1a , #D4AF37," }))).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["color"]["name"], "Black + Gold");
    assert_eq!(body["color"]["hex"], "#1a1a1a, #D4AF37", "parts trimmed, empty ones dropped");
}

// spec: 007/FR-015
#[tokio::test]
async fn renaming_a_color_carries_its_filaments_along() {
    let pool = test_pool().await;
    let filament = add_filament(&pool, "Black").await;
    add_color(&pool, "White", "#FFFFFF").await;
    let black: i64 = sqlx::query_scalar("SELECT id FROM filament_colors WHERE name = 'Black'").fetch_one(&pool).await.unwrap();

    let (status, body) = send(&pool, "PUT", &format!("/api/filament-colors/{black}"), Some(serde_json::json!({ "name": "White", "hex": "#000000" }))).await;
    assert_eq!((status, body["error"].clone()), (StatusCode::BAD_REQUEST, serde_json::json!("duplicate")));
    let (status, _) = send(&pool, "PUT", "/api/filament-colors/999", Some(serde_json::json!({ "name": "X", "hex": "#000000" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = send(&pool, "PUT", &format!("/api/filament-colors/{black}"), Some(serde_json::json!({ "name": "Jet Black", "hex": "#0A0A0A" }))).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let color: String = sqlx::query_scalar("SELECT color FROM filaments WHERE id = ?1").bind(filament).fetch_one(&pool).await.unwrap();
    assert_eq!(color, "Jet Black");
}

// spec: 007/FR-016
#[tokio::test]
async fn delete_is_refused_while_a_filament_uses_the_color() {
    let pool = test_pool().await;
    add_filament(&pool, "Black").await;
    let black: i64 = sqlx::query_scalar("SELECT id FROM filament_colors WHERE name = 'Black'").fetch_one(&pool).await.unwrap();
    let unused = add_color(&pool, "Teal", "#008080").await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/filament-colors/{black}"), None).await;
    assert_eq!((status, body["error"].clone()), (StatusCode::BAD_REQUEST, serde_json::json!("in_use")));
    let (status, _) = send(&pool, "DELETE", &format!("/api/filament-colors/{unused}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(&pool, "DELETE", &format!("/api/filament-colors/{unused}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// spec: 007/FR-017
#[tokio::test]
async fn inventory_filters_sorts_and_pages() {
    let pool = test_pool().await;
    for (name, hex) in [("Sky Blue", "#87CEEB"), ("Navy Blue", "#000080"), ("Red", "#FF0000"), ("Blue Grey", "#6699CC")] {
        add_color(&pool, name, hex).await;
    }

    let (status, body) = send(&pool, "GET", "/api/filament-colors/inventory?q=BLUE&sort=hex&order=desc&page=1&pageSize=2", None).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["total"], 3);
    assert_eq!(body["totalPages"], 2);
    let names: Vec<&str> = body["colors"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Sky Blue", "Blue Grey"]);

    let (_, body) = send(&pool, "GET", "/api/filament-colors/inventory?pageSize=0", None).await;
    assert_eq!(body["pageSize"], 20, "non-positive falls back to the default");
    assert_eq!(body["colors"][0]["name"], "Blue Grey", "default sort: name ascending");
}
