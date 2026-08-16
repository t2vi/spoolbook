pub mod colors;
pub mod filaments;
pub mod spools;

use axum::Router;
use sqlx::SqlitePool;

pub fn app(pool: SqlitePool) -> Router {
    filaments::router()
        .merge(colors::router())
        .merge(spools::router())
        .with_state(pool)
}
