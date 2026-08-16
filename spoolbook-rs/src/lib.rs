pub mod colors;
pub mod dashboard;
pub mod filaments;
pub mod printers;
pub mod prints;
pub mod profile_field_spec;
pub mod profiles;
pub mod projects;
pub mod settings;
pub mod spools;

use axum::Router;
use sqlx::SqlitePool;

pub fn app(pool: SqlitePool) -> Router {
    filaments::router()
        .merge(colors::router())
        .merge(spools::router())
        .merge(profiles::router())
        .merge(printers::router())
        .merge(projects::router())
        .merge(prints::router())
        .merge(settings::router())
        .merge(dashboard::router())
        .with_state(pool)
}
