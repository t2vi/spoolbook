pub mod bambu_import;
pub mod bambu_mqtt_payload_parser;
pub mod colors;
pub mod dashboard;
pub mod filament_catalog_sync;
pub mod filaments;
pub mod printers;
pub mod prints;
pub mod printer_mqtt;
pub mod printer_telemetry;
pub mod profile_config_patcher;
pub mod profile_field_spec;
pub mod profiles;
pub mod project_upload;
pub mod projects;
pub mod reslicing;
pub mod send_print;
pub mod settings;
pub mod spools;

use axum::{Extension, Router};
use sqlx::SqlitePool;

// Tests (and anything else that doesn't care about live MQTT status) get a fresh, empty store —
// the /live endpoint just reports "not connected" rather than needing a real background
// connection. Production wiring (main.rs) uses app_with_live_status with the real, populated one.
pub fn app(pool: SqlitePool) -> Router {
    app_with_live_status(pool, printer_mqtt::new_store())
}

pub fn app_with_live_status(pool: SqlitePool, live_status: printer_mqtt::LiveStatusStore) -> Router {
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
        .merge(reslicing::router())
        .merge(send_print::router())
        .layer(Extension(live_status))
        .with_state(pool)
}
