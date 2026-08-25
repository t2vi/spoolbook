use serde_json::json;
use spoolbook_rs::weather::fetch_ambient_average;
use tokio::net::TcpListener;

// ARCHIVE_URL is process-global env state; every test in this file holds this lock for its whole
// request (tests in other files run in separate processes, unaffected) -- same pattern
// tests/reslicing.rs and tests/google_oauth.rs already use for their own env-var overrides.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn set_archive_url(url: &str) {
    unsafe { std::env::set_var("ARCHIVE_URL", url) };
}

async fn spawn_mock_archive(response: serde_json::Value) -> String {
    use axum::Json;
    use axum::extract::State;

    async fn archive(State(body): State<serde_json::Value>) -> Json<serde_json::Value> {
        Json(body)
    }

    let app = axum::Router::new().route("/v1/archive", axum::routing::get(archive)).with_state(response);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/v1/archive")
}

#[tokio::test]
async fn averages_only_the_hours_within_the_print_window() {
    let _guard = ENV_LOCK.lock().unwrap();
    let response = json!({
        "hourly": {
            "time": ["2026-01-01T06:00", "2026-01-01T07:00", "2026-01-01T08:00", "2026-01-01T09:00", "2026-01-01T10:00"],
            "temperature_2m": [10.0, 20.0, 22.0, 24.0, 30.0],
            "relative_humidity_2m": [40.0, 50.0, 52.0, 54.0, 60.0]
        }
    });
    set_archive_url(&spawn_mock_archive(response).await);

    // Print ran 07:00-09:00 -- only those three hours (20/22/24, 50/52/54) should count.
    let (temp, humidity) = fetch_ambient_average(-37.8, 144.9, "2026-01-01T07:00:00Z", "2026-01-01T09:00:00Z").await.unwrap();
    assert!((temp - 22.0).abs() < 0.01, "{temp}");
    assert!((humidity - 52.0).abs() < 0.01, "{humidity}");
}

#[tokio::test]
async fn errors_when_no_hourly_reading_falls_in_the_print_window() {
    let _guard = ENV_LOCK.lock().unwrap();
    let response = json!({
        "hourly": {
            "time": ["2026-01-01T06:00"],
            "temperature_2m": [10.0],
            "relative_humidity_2m": [40.0]
        }
    });
    set_archive_url(&spawn_mock_archive(response).await);

    let result = fetch_ambient_average(-37.8, 144.9, "2026-06-01T07:00:00Z", "2026-06-01T09:00:00Z").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn errors_when_the_archive_endpoint_is_unreachable() {
    let _guard = ENV_LOCK.lock().unwrap();
    set_archive_url("http://127.0.0.1:1/v1/archive");
    let result = fetch_ambient_average(-37.8, 144.9, "2026-01-01T07:00:00Z", "2026-01-01T09:00:00Z").await;
    assert!(result.is_err());
}
