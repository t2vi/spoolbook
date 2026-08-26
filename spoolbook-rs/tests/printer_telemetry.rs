use spoolbook_rs::bambu_mqtt_payload_parser::ReadingInput;
use spoolbook_rs::printer_telemetry::{attach_job_to_print, end_job, find_match_for_print, purge_unattached_jobs_older_than, record_reading};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

// ARCHIVE_URL is process-global env state; every test in this file that touches it holds this
// lock for its whole request sequence (tests in other files run in separate processes,
// unaffected) -- same pattern tests/reslicing.rs and tests/google_oauth.rs already use.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn spawn_mock_archive() -> String {
    use axum::Json;
    use tokio::net::TcpListener;

    async fn archive() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "hourly": {
                "time": ["2026-01-01T07:00", "2026-01-01T08:00", "2026-01-01T09:00"],
                "temperature_2m": [20.0, 22.0, 24.0],
                "relative_humidity_2m": [50.0, 52.0, 54.0]
            }
        }))
    }

    let app = axum::Router::new().route("/v1/archive", axum::routing::get(archive));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/v1/archive")
}

async fn set_location(pool: &sqlx::SqlitePool, latitude: f64, longitude: f64) {
    sqlx::query("INSERT INTO app_settings (id) VALUES (1)").execute(pool).await.ok();
    sqlx::query("UPDATE app_settings SET latitude = ?1, longitude = ?2 WHERE id = 1").bind(latitude).bind(longitude).execute(pool).await.unwrap();
}

async fn test_pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn seed_printer(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("INSERT INTO printers (name) VALUES ('Garage P2S') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_print_deps(pool: &sqlx::SqlitePool) -> (i64, i64) {
    let filament_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO filaments (brand, material, variant, color) VALUES ('Bambu Lab', 'PLA', 'Basic', 'Black') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let spool_id = sqlx::query_scalar::<_, i64>("INSERT INTO spools (filament_id) VALUES (?1) RETURNING id")
        .bind(filament_id)
        .fetch_one(pool)
        .await
        .unwrap();
    let profile_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO print_profiles (filament_id, name, nozzle_temp_c) VALUES (?1, 'Standard', 230) RETURNING id",
    )
    .bind(filament_id)
    .fetch_one(pool)
    .await
    .unwrap();
    (profile_id, spool_id)
}

async fn seed_in_progress_print(pool: &sqlx::SqlitePool, profile_id: i64, spool_id: i64, printer_id: i64, started_at: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO prints (profile_id, spool_id, printer_id, started_at, status) VALUES (?1, ?2, ?3, ?4, 'InProgress') RETURNING id",
    )
    .bind(profile_id)
    .bind(spool_id)
    .bind(printer_id)
    .bind(started_at)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn reading(nozzle: f64) -> ReadingInput {
    ReadingInput { nozzle_temp_c: Some(nozzle), bed_temp_c: None, chamber_temp_c: None, ams_slot: None, progress_pct: None }
}

async fn job_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM printer_jobs").fetch_one(pool).await.unwrap()
}

async fn reading_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM printer_readings").fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn record_reading_creates_new_job_on_first_reading_for_external_id() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;

    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    assert_eq!(job_count(&pool).await, 1);
    assert_eq!(reading_count(&pool).await, 1);
    let external_id = sqlx::query_scalar::<_, String>("SELECT external_job_id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert_eq!(external_id, "job-1");
}

#[tokio::test]
async fn record_reading_appends_reading_to_existing_active_job() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;

    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;
    record_reading(&pool, printer_id, "job-1", &reading(248.0), None).await;

    assert_eq!(job_count(&pool).await, 1);
    assert_eq!(reading_count(&pool).await, 2);
}

#[tokio::test]
async fn record_reading_different_external_ids_create_separate_jobs() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;

    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;
    record_reading(&pool, printer_id, "job-2", &reading(245.0), None).await;

    assert_eq!(job_count(&pool).await, 2);
}

#[tokio::test]
async fn end_job_sets_ended_at() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    end_job(&pool, printer_id, "job-1", None, None, None, None, &camera_registry).await;

    let ended_at: Option<String> = sqlx::query_scalar("SELECT ended_at FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert!(ended_at.is_some());
}

#[tokio::test]
async fn end_job_snapshots_chamber_temp_and_ams_humidity_onto_the_print() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    let print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T08:00:00Z").await;
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    end_job(&pool, printer_id, "job-1", Some("FINISH"), None, Some(38.5), Some(21), &camera_registry).await;

    let (chamber_temp_c, ams_humidity_pct): (Option<f64>, Option<i64>) =
        sqlx::query_as("SELECT chamber_temp_c, ams_humidity_pct FROM prints WHERE id = ?1").bind(print_id).fetch_one(&pool).await.unwrap();
    assert!((chamber_temp_c.unwrap() - 38.5).abs() < 0.01);
    assert_eq!(ams_humidity_pct, Some(21));
}

#[tokio::test]
async fn record_reading_auto_attaches_new_job_to_open_in_progress_print_for_same_printer() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    let print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T08:00:00Z").await;

    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    let job_print_id: Option<i64> = sqlx::query_scalar("SELECT print_id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert_eq!(job_print_id, Some(print_id));
}

#[tokio::test]
async fn record_reading_does_not_reattach_on_subsequent_readings_for_same_job() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    let print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T08:00:00Z").await;
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    // A second, unrelated InProgress print shows up before the next reading — the already-
    // attached job must not be reassigned to it.
    let second_print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T09:00:00Z").await;
    record_reading(&pool, printer_id, "job-1", &reading(248.0), None).await;

    let job_print_id: Option<i64> = sqlx::query_scalar("SELECT print_id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert_eq!(job_print_id, Some(print_id));
    assert_ne!(job_print_id, Some(second_print_id));
}

#[tokio::test]
async fn record_reading_does_not_auto_attach_when_no_in_progress_print_exists() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;

    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    let job_print_id: Option<i64> = sqlx::query_scalar("SELECT print_id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert_eq!(job_print_id, None);
}

#[tokio::test]
async fn end_job_sets_attached_print_status_from_terminal_gcode_state() {
    for (gcode_state, expected_status) in [
        (Some("FINISH"), "Success"),
        (Some("FAILED"), "Failed"),
        (Some("IDLE"), "Partial"),
        (None, "Partial"),
    ] {
        let pool = test_pool().await;
        let printer_id = seed_printer(&pool).await;
        let camera_registry = spoolbook_rs::printer_camera::new_registry();
        let (profile_id, spool_id) = seed_print_deps(&pool).await;
        let print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T08:00:00Z").await;
        record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

        end_job(&pool, printer_id, "job-1", gcode_state, None, None, None, &camera_registry).await;

        let (status, ended_at): (String, Option<String>) =
            sqlx::query_as("SELECT status, ended_at FROM prints WHERE id = ?1").bind(print_id).fetch_one(&pool).await.unwrap();
        assert_eq!(status, expected_status, "gcode_state {gcode_state:?}");
        assert!(ended_at.is_some());
    }
}

#[tokio::test]
async fn end_job_fetches_and_stores_ambient_weather_when_location_is_configured() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = test_pool().await;
    unsafe { std::env::set_var("ARCHIVE_URL", spawn_mock_archive().await) };
    set_location(&pool, -37.8, 144.9).await;
    let printer_id = seed_printer(&pool).await;
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    let print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T07:00:00Z").await;
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    end_job(&pool, printer_id, "job-1", Some("FINISH"), Some("2026-01-01T09:00:00Z"), None, None, &camera_registry).await;

    let (temp, humidity, source): (Option<f64>, Option<f64>, Option<String>) =
        sqlx::query_as("SELECT ambient_temp_c, ambient_humidity_pct, ambient_source FROM prints WHERE id = ?1").bind(print_id).fetch_one(&pool).await.unwrap();
    assert!((temp.unwrap() - 22.0).abs() < 0.01, "{temp:?}");
    assert!((humidity.unwrap() - 52.0).abs() < 0.01, "{humidity:?}");
    assert_eq!(source.as_deref(), Some("open-meteo"));
}

#[tokio::test]
async fn end_job_persists_every_hour_of_the_print_window_for_the_graph() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = test_pool().await;
    unsafe { std::env::set_var("ARCHIVE_URL", spawn_mock_archive().await) };
    set_location(&pool, -37.8, 144.9).await;
    let printer_id = seed_printer(&pool).await;
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    let print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T07:00:00Z").await;
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    end_job(&pool, printer_id, "job-1", Some("FINISH"), Some("2026-01-01T09:00:00Z"), None, None, &camera_registry).await;

    let rows: Vec<(String, Option<f64>, Option<f64>)> =
        sqlx::query_as("SELECT hour, temp_c, humidity_pct FROM print_hourly_weather WHERE print_id = ?1 ORDER BY hour")
            .bind(print_id)
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0].0, "2026-01-01T07:00");
    assert!((rows[0].1.unwrap() - 20.0).abs() < 0.01);
    assert!((rows[0].2.unwrap() - 50.0).abs() < 0.01);
    assert_eq!(rows[2].0, "2026-01-01T09:00");
    assert!((rows[2].1.unwrap() - 24.0).abs() < 0.01);
}

#[tokio::test]
async fn end_job_leaves_ambient_weather_null_when_no_location_is_configured() {
    let _guard = ENV_LOCK.lock().unwrap();
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    let print_id = seed_in_progress_print(&pool, profile_id, spool_id, printer_id, "2026-01-01T07:00:00Z").await;
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    end_job(&pool, printer_id, "job-1", Some("FINISH"), Some("2026-01-01T09:00:00Z"), None, None, &camera_registry).await;

    let (temp, source): (Option<f64>, Option<String>) =
        sqlx::query_as("SELECT ambient_temp_c, ambient_source FROM prints WHERE id = ?1").bind(print_id).fetch_one(&pool).await.unwrap();
    assert_eq!(temp, None);
    assert_eq!(source, None);
}

#[tokio::test]
async fn end_job_does_not_touch_print_status_when_job_has_no_attached_print() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let camera_registry = spoolbook_rs::printer_camera::new_registry();
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;

    end_job(&pool, printer_id, "job-1", Some("FINISH"), None, None, None, &camera_registry).await;

    let (ended_at, print_id): (Option<String>, Option<i64>) =
        sqlx::query_as("SELECT ended_at, print_id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    assert!(ended_at.is_some());
    assert_eq!(print_id, None);
}

#[tokio::test]
async fn find_match_for_print_returns_closest_unattached_job_by_start_time() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    record_reading(&pool, printer_id, "job-far", &reading(245.0), Some("2026-01-01T06:00:00Z")).await;
    record_reading(&pool, printer_id, "job-close", &reading(245.0), Some("2026-01-01T08:00:00Z")).await;

    let matched = find_match_for_print(&pool, printer_id, "2026-01-01T08:05:00Z").await;

    assert_eq!(matched.unwrap().external_job_id, "job-close");
}

#[tokio::test]
async fn find_match_for_print_excludes_already_attached_jobs() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    record_reading(&pool, printer_id, "job-1", &reading(245.0), Some("2026-01-01T08:00:00Z")).await;
    let job_id = sqlx::query_scalar::<_, i64>("SELECT id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    let print_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO prints (profile_id, spool_id, printer_id, started_at, ended_at, status)
         VALUES (?1, ?2, ?3, '2026-01-01T08:00:00Z', '2026-01-01T10:00:00Z', 'Success') RETURNING id",
    )
    .bind(profile_id)
    .bind(spool_id)
    .bind(printer_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    attach_job_to_print(&pool, job_id, print_id).await;

    let matched = find_match_for_print(&pool, printer_id, "2026-01-01T08:00:00Z").await;

    assert!(matched.is_none());
}

#[tokio::test]
async fn find_match_for_print_returns_none_when_no_unattached_jobs_exist() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;

    let matched = find_match_for_print(&pool, printer_id, "2026-01-01T08:00:00Z").await;

    assert!(matched.is_none());
}

#[tokio::test]
async fn attach_job_to_print_sets_print_id() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    record_reading(&pool, printer_id, "job-1", &reading(245.0), None).await;
    let job_id = sqlx::query_scalar::<_, i64>("SELECT id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    let print_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO prints (profile_id, spool_id, printer_id, started_at, ended_at, status)
         VALUES (?1, ?2, ?3, '2026-01-01T08:00:00Z', '2026-01-01T10:00:00Z', 'Success') RETURNING id",
    )
    .bind(profile_id)
    .bind(spool_id)
    .bind(printer_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    attach_job_to_print(&pool, job_id, print_id).await;

    let updated: Option<i64> = sqlx::query_scalar("SELECT print_id FROM printer_jobs WHERE id = ?1").bind(job_id).fetch_one(&pool).await.unwrap();
    assert_eq!(updated, Some(print_id));
}

#[tokio::test]
async fn purge_unattached_jobs_older_than_removes_old_unattached_jobs_and_their_readings() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    record_reading(&pool, printer_id, "old-job", &reading(245.0), Some("2026-01-01T08:00:00Z")).await;

    purge_unattached_jobs_older_than(&pool, "2026-01-05T00:00:00Z").await;

    assert_eq!(job_count(&pool).await, 0);
    assert_eq!(reading_count(&pool).await, 0, "readings must cascade-delete with their job");
}

#[tokio::test]
async fn purge_unattached_jobs_older_than_keeps_attached_jobs_regardless_of_age() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    let (profile_id, spool_id) = seed_print_deps(&pool).await;
    record_reading(&pool, printer_id, "old-job", &reading(245.0), Some("2026-01-01T08:00:00Z")).await;
    let job_id = sqlx::query_scalar::<_, i64>("SELECT id FROM printer_jobs").fetch_one(&pool).await.unwrap();
    let print_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO prints (profile_id, spool_id, printer_id, started_at, ended_at, status)
         VALUES (?1, ?2, ?3, '2026-01-01T08:00:00Z', '2026-01-01T10:00:00Z', 'Success') RETURNING id",
    )
    .bind(profile_id)
    .bind(spool_id)
    .bind(printer_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    attach_job_to_print(&pool, job_id, print_id).await;

    purge_unattached_jobs_older_than(&pool, "2026-01-05T00:00:00Z").await;

    assert_eq!(job_count(&pool).await, 1);
}

#[tokio::test]
async fn purge_unattached_jobs_older_than_keeps_recent_unattached_jobs() {
    let pool = test_pool().await;
    let printer_id = seed_printer(&pool).await;
    record_reading(&pool, printer_id, "recent-job", &reading(245.0), Some("2026-01-06T08:00:00Z")).await;

    purge_unattached_jobs_older_than(&pool, "2026-01-05T00:00:00Z").await;

    assert_eq!(job_count(&pool).await, 1);
}
