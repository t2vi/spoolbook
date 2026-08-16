use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FilamentColor {
    pub id: i64,
    pub name: String,
    pub hex: String,
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/filament-colors", get(list))
}

async fn list(State(pool): State<SqlitePool>) -> Json<Vec<FilamentColor>> {
    let colors = sqlx::query_as::<_, FilamentColor>("SELECT id, name, hex FROM filament_colors ORDER BY name")
        .fetch_all(&pool)
        .await
        .expect("query failed");

    Json(colors)
}
