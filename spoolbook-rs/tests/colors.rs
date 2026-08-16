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

#[tokio::test]
async fn list_colors_returns_empty_array_when_none_seeded() {
    let pool = test_pool().await;
    let app = spoolbook_rs::app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/filament-colors")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body.as_ref(), b"[]");
}

#[tokio::test]
async fn list_colors_returns_seeded_rows_sorted_by_name() {
    let pool = test_pool().await;
    sqlx::query("INSERT INTO filament_colors (name, hex) VALUES ('White', '#FFFFFF'), ('Black', '#000000')")
        .execute(&pool)
        .await
        .unwrap();

    let app = spoolbook_rs::app(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/filament-colors")
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
