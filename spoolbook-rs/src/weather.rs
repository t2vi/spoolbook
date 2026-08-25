// Ambient weather auto-fetch (github.com/t2vi/spoolbook/issues/94): once a print ends, pull the
// hourly outdoor temperature/humidity for the print's [started_at, ended_at] window from
// Open-Meteo's historical/archive API (ERA5 reanalysis -- hourly is its finest resolution, no
// reliable sub-hourly for an arbitrary location) and average it into prints.ambient_temp_c /
// ambient_humidity_pct. This is a separate, single-location value from the existing manual
// actual_room_temp_c (indoor, near the printer) -- ambient stays informational/correlation-only,
// never folded into recommend_profile's ranking, and is never hand-typed: the only way in today
// is this fetch (auto_source records that), with a future thermometer integration as a possible
// second source.
use chrono::{DateTime, NaiveDateTime};
use serde::Deserialize;
use sqlx::SqlitePool;

fn archive_url() -> String {
    std::env::var("ARCHIVE_URL").unwrap_or_else(|_| "https://archive-api.open-meteo.com/v1/archive".to_string())
}

#[derive(Deserialize)]
struct ArchiveResponse {
    hourly: HourlyData,
}

#[derive(Deserialize)]
struct HourlyData {
    time: Vec<String>,
    temperature_2m: Vec<Option<f64>>,
    relative_humidity_2m: Vec<Option<f64>>,
}

// Averages only the hours that actually fall within the print's [started_at, ended_at] window --
// the archive API only takes whole-day date params, so a short print still gets its own window's
// readings, not the whole day's.
pub async fn fetch_ambient_average(latitude: f64, longitude: f64, started_at: &str, ended_at: &str) -> Result<(f64, f64), String> {
    let start = DateTime::parse_from_rfc3339(started_at).map_err(|e| e.to_string())?.naive_utc();
    let end = DateTime::parse_from_rfc3339(ended_at).map_err(|e| e.to_string())?.naive_utc();

    // Built via reqwest::Url's own query-pair encoder rather than RequestBuilder::query (not
    // available with this crate's trimmed feature set) -- same approach google_oauth.rs uses.
    let mut url = reqwest::Url::parse(&archive_url()).map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("latitude", &latitude.to_string())
        .append_pair("longitude", &longitude.to_string())
        .append_pair("start_date", &start.format("%Y-%m-%d").to_string())
        .append_pair("end_date", &end.format("%Y-%m-%d").to_string())
        .append_pair("hourly", "temperature_2m,relative_humidity_2m")
        .append_pair("timezone", "UTC");

    let resp = reqwest::Client::new().get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("archive fetch failed: {}", resp.status()));
    }
    let body: ArchiveResponse = resp.json().await.map_err(|e| e.to_string())?;

    let mut temps = Vec::new();
    let mut humidities = Vec::new();
    for i in 0..body.hourly.time.len() {
        let Ok(hour) = NaiveDateTime::parse_from_str(&body.hourly.time[i], "%Y-%m-%dT%H:%M") else { continue };
        if hour < start || hour > end {
            continue;
        }
        if let Some(t) = body.hourly.temperature_2m.get(i).copied().flatten() {
            temps.push(t);
        }
        if let Some(h) = body.hourly.relative_humidity_2m.get(i).copied().flatten() {
            humidities.push(h);
        }
    }

    if temps.is_empty() || humidities.is_empty() {
        return Err("no hourly reading falls within the print window".to_string());
    }
    let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    Ok((avg(&temps), avg(&humidities)))
}

// Called from printer_telemetry::end_job right after a print's ended_at is set. Fire-and-forget
// in effect (never panics, never propagates an error to the caller) -- same posture as
// filament_catalog_sync's startup sync. A missing location, a missing/malformed print row, or a
// failed fetch all just leave ambient_temp_c/ambient_humidity_pct/ambient_source null; there's no
// retry, this is enrichment data for long-run correlation, not something a single print's history
// depends on.
pub async fn fetch_and_store(pool: &SqlitePool, print_id: i64) {
    let settings = crate::settings::fetch(pool).await;
    let (Some(latitude), Some(longitude)) = (settings.latitude, settings.longitude) else { return };

    let Ok(Some((started_at, ended_at))) =
        sqlx::query_as::<_, (String, Option<String>)>("SELECT started_at, ended_at FROM prints WHERE id = ?1").bind(print_id).fetch_optional(pool).await
    else {
        return;
    };
    let Some(ended_at) = ended_at else { return };

    match fetch_ambient_average(latitude, longitude, &started_at, &ended_at).await {
        Ok((temp, humidity)) => {
            sqlx::query("UPDATE prints SET ambient_temp_c = ?1, ambient_humidity_pct = ?2, ambient_source = 'open-meteo' WHERE id = ?3")
                .bind(temp)
                .bind(humidity)
                .bind(print_id)
                .execute(pool)
                .await
                .expect("update failed");
        }
        Err(e) => eprintln!("ambient weather fetch failed for print {print_id}: {e}"),
    }
}
