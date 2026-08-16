use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::io::Write;
use tower::ServiceExt;

async fn post_multipart(uri: &str, field_name: &str, filename: &str, content: &[u8]) -> (StatusCode, Value) {
    let boundary = "spoolbook-rs-test-boundary";
    let mut body = Vec::new();
    write!(
        body,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .unwrap();
    body.extend_from_slice(content);
    write!(body, "\r\n--{boundary}--\r\n").unwrap();

    let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let response = spoolbook_rs::app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", format!("multipart/form-data; boundary={boundary}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

// A minimal but real .3mf zip carrying Metadata/project_settings.config, the same baked-preset
// shape Bambu Studio writes at slice time (docs/adr/0018).
fn fake_3mf_with_settings(settings_json: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("Metadata/project_settings.config", options).unwrap();
        zip.write_all(settings_json.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf
}

#[tokio::test]
async fn imports_temperature_bool_and_percent_fields_from_a_real_3mf() {
    let settings = r##"{
        "nozzle_temperature": ["220"],
        "nozzle_temperature_initial_layer": ["215"],
        "enable_overhang_bridge_fan": ["1"],
        "filament_soluble": ["0"],
        "filament_shrink": ["100%"],
        "default_filament_colour": ["#112233"],
        "unmapped_bambu_key": ["ignored"]
    }"##;
    let bytes = fake_3mf_with_settings(settings);

    let (status, body) = post_multipart("/api/profiles/import-3mf", "file", "Cold Day Tune.3mf", &bytes).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["suggestedName"], "Cold Day Tune");
    assert_eq!(body["fields"]["NozzleTempC"], "220");
    assert_eq!(body["fields"]["NozzleTempInitialC"], "215");
    assert_eq!(body["fields"]["EnableOverhangBridgeFan"], "true", "bool field: raw \"1\" -> \"true\"");
    assert_eq!(body["fields"]["Soluble"], "false", "bool field: raw \"0\" -> \"false\"");
    assert_eq!(body["fields"]["ShrinkPct"], "100", "percent-suffix field strips the trailing %");
    assert_eq!(body["fields"]["DefaultColourHex"], "#112233");
    assert!(body["fields"].get("unmapped_bambu_key").is_none());
    assert!(body["rawSettingsJson"].as_str().unwrap().contains("nozzle_temperature"));
}

#[tokio::test]
async fn treats_nil_and_empty_string_values_as_absent() {
    let settings = r#"{ "nozzle_temperature": ["nil"], "filament_notes": [""] }"#;
    let bytes = fake_3mf_with_settings(settings);

    let (_, body) = post_multipart("/api/profiles/import-3mf", "file", "x.3mf", &bytes).await;

    assert!(body["fields"].get("NozzleTempC").is_none());
    assert!(body["fields"].get("SlicerNotes").is_none());
}

#[tokio::test]
async fn returns_no_project_settings_when_the_config_entry_is_missing() {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        zip.start_file("3D/3dmodel.model", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"<mesh/>").unwrap();
        zip.finish().unwrap();
    }

    let (status, body) = post_multipart("/api/profiles/import-3mf", "file", "x.3mf", &buf).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "no_project_settings");
}

#[tokio::test]
async fn returns_invalid_3mf_for_a_non_zip_upload() {
    let (status, body) = post_multipart("/api/profiles/import-3mf", "file", "x.3mf", b"not a zip").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_3mf");
}

#[tokio::test]
async fn returns_bad_request_when_no_file_field_is_present() {
    let (status, body) = post_multipart("/api/profiles/import-3mf", "not_file", "x.3mf", b"junk").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "No file provided.");
}
