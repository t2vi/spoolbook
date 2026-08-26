use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    // SPOOLBOOK_DB_PATH overrides this for production (Docker/LXC, where it points at the
    // mounted data volume) or a second local instance run alongside a live one. Unset defaults
    // to the relative dev.db every earlier session in this repo already assumes.
    let db_path = std::env::var("SPOOLBOOK_DB_PATH").unwrap_or_else(|_| "dev.db".to_string());
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}?mode=rwc"))
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap_or_else(|e| panic!("failed to open {db_path}: {e}"));
    sqlx::migrate!().run(&pool).await.expect("migration failed");

    // Existing installs running on SPOOLBOOK_ADMIN_PASSWORD before real user accounts shipped
    // get migrated seamlessly (no wizard); a genuinely fresh install with no users and no env
    // var falls through and the frontend shows the setup wizard (GET /api/setup-status).
    spoolbook_rs::auth::migrate_env_var_admin_if_needed(&pool).await;

    // Converts any already-finished Print's raw Readings into its telemetry_json snapshot --
    // covers Prints that finished before docs/adr/0032 shipped. Idempotent, same posture as the
    // admin migration above.
    spoolbook_rs::printer_telemetry::backfill_reading_snapshots(&pool).await;

    let live_status = spoolbook_rs::printer_mqtt::new_store();
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    spoolbook_rs::printer_mqtt::spawn_all(pool.clone(), live_status.clone(), camera_registry.clone()).await;

    // Throttled to once/24h via app_settings.last_filament_sync_at, same as the .NET app's
    // Program.cs startup block — silent on failure, the Filaments page's manual sync button
    // surfaces errors for an explicit attempt.
    {
        let settings = spoolbook_rs::settings::fetch(&pool).await;
        if spoolbook_rs::filament_catalog_sync::should_sync(settings.last_filament_sync_at.as_deref(), chrono::Utc::now()) {
            let sync_pool = pool.clone();
            tokio::spawn(async move {
                let _ = spoolbook_rs::filament_catalog_sync::run_sync(&sync_pool).await;
            });
        }
    }

    // spoolbook-web-svelte's static build (adapter-static output) — a sibling checkout
    // directory, not copied in, so a frontend rebuild doesn't require rebuilding this crate.
    // SPOOLBOOK_STATIC_ROOT overrides this for Docker/LXC packaging, where the build output
    // won't sit next to this crate on disk the way it does in a plain repo pull. Anything not
    // matched by an API route above falls back to index.html, letting SvelteKit's own
    // client-side router take over (e.g. /prints/edit/5) — plain `.fallback()`, not
    // `.not_found_service()`, so the response is 200 (matching MapFallbackToFile's own status),
    // not a 404 the browser would treat as a failed navigation.
    let static_root =
        std::env::var("SPOOLBOOK_STATIC_ROOT").unwrap_or_else(|_| "../spoolbook-web-svelte/build".to_string());
    let index_path = std::path::Path::new(&static_root).join("index.html");
    let serve_static = ServeDir::new(&static_root).fallback(ServeFile::new(index_path));

    let app = spoolbook_rs::app_with_camera(pool, live_status, camera_registry).fallback_service(serve_static);

    // 0.0.0.0, not 127.0.0.1: matches .NET's ASPNETCORE_URLS=http://0.0.0.0:5070 (self-hosted,
    // LAN-accessible per CLAUDE.md) and is required for Docker port publishing to reach it at all.
    let listener = TcpListener::bind("0.0.0.0:5070").await.unwrap();
    println!("spoolbook-rs listening on http://0.0.0.0:5070");
    axum::serve(listener, app).await.unwrap();
}
