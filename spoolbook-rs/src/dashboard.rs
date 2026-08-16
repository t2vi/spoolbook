use crate::settings;
use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Serialize, sqlx::FromRow)]
struct CategoryCount {
    label: String,
    count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardMetrics {
    filament_count: i64,
    last_filament_sync_at: Option<String>,
    filaments_by_brand: Vec<CategoryCount>,
    filaments_by_material: Vec<CategoryCount>,
    spools_by_status: Vec<CategoryCount>,
    prints_by_status: Vec<CategoryCount>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardSnapshot {
    metrics: DashboardMetrics,
    profile_count: i64,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/dashboard", get(dashboard))
}

async fn grouped_counts(pool: &SqlitePool, column: &str) -> Vec<CategoryCount> {
    let sql = format!("SELECT {column} AS label, COUNT(*) AS count FROM filaments GROUP BY {column} ORDER BY count DESC");
    sqlx::query_as::<_, CategoryCount>(&sql).fetch_all(pool).await.expect("query failed")
}

async fn dashboard(State(pool): State<SqlitePool>) -> Json<DashboardSnapshot> {
    let filament_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM filaments")
        .fetch_one(&pool)
        .await
        .expect("query failed");

    let filaments_by_brand = grouped_counts(&pool, "brand").await;
    let filaments_by_material = grouped_counts(&pool, "material").await;

    let unopened = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM spools WHERE opened_at IS NULL")
        .fetch_one(&pool)
        .await
        .expect("query failed");
    let opened = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM spools WHERE opened_at IS NOT NULL AND emptied_at IS NULL")
        .fetch_one(&pool)
        .await
        .expect("query failed");
    let empty = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM spools WHERE emptied_at IS NOT NULL")
        .fetch_one(&pool)
        .await
        .expect("query failed");
    let spools_by_status = vec![
        CategoryCount { label: "Unopened".into(), count: unopened },
        CategoryCount { label: "Opened".into(), count: opened },
        CategoryCount { label: "Empty".into(), count: empty },
    ];

    // Fixed order/labels (not GROUP BY) so a status with zero prints still appears, matching
    // .NET's Enum.GetValues(PrintStatus) walk.
    let mut prints_by_status = Vec::with_capacity(4);
    for status in ["Success", "Failed", "Partial", "InProgress"] {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM prints WHERE status = ?1")
            .bind(status)
            .fetch_one(&pool)
            .await
            .expect("query failed");
        prints_by_status.push(CategoryCount { label: status.into(), count });
    }

    let app_settings = settings::fetch(&pool).await;

    let profile_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM print_profiles WHERE is_current_version = 1")
        .fetch_one(&pool)
        .await
        .expect("query failed");

    Json(DashboardSnapshot {
        metrics: DashboardMetrics {
            filament_count,
            last_filament_sync_at: app_settings.last_filament_sync_at,
            filaments_by_brand,
            filaments_by_material,
            spools_by_status,
            prints_by_status,
        },
        profile_count,
    })
}
