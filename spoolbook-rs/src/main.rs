use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    let options = SqliteConnectOptions::from_str("sqlite://dev.db?mode=rwc")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("failed to open dev.db");
    sqlx::migrate!().run(&pool).await.expect("migration failed");

    let live_status = spoolbook_rs::printer_mqtt::new_store();
    spoolbook_rs::printer_mqtt::spawn_all(pool.clone(), live_status.clone()).await;
    let camera_registry = spoolbook_rs::printer_camera::new_registry();

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

    let listener = TcpListener::bind("127.0.0.1:8090").await.unwrap();
    println!("spoolbook-rs listening on http://127.0.0.1:8090");
    axum::serve(listener, app).await.unwrap();
}
