pub mod bambu_import;
pub mod bambu_mqtt_payload_parser;
pub mod colors;
pub mod dashboard;
pub mod filament_catalog_sync;
pub mod filaments;
pub mod printers;
pub mod prints;
pub mod printer_telemetry;
pub mod profile_field_spec;
pub mod profiles;
pub mod project_upload;
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
        .merge(filament_catalog_sync::router())
        .merge(project_upload::router())
        .merge(bambu_import::router())
        .with_state(pool)
}
