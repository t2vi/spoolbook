mod common;

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

    let cookie = common::auth_cookie_header(&pool).await;
    let response = spoolbook_rs::app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", format!("multipart/form-data; boundary={boundary}"))
                .header("cookie", cookie)
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

    let (status, body) = post_multipart("/api/profiles/import-preset", "file", "Cold Day Tune.3mf", &bytes).await;

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

    let (_, body) = post_multipart("/api/profiles/import-preset", "file", "x.3mf", &bytes).await;

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

    let (status, body) = post_multipart("/api/profiles/import-preset", "file", "x.3mf", &buf).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "no_project_settings");
}

// Neither a zip nor valid JSON -- fails both the .3mf and the raw-preset fallback.
#[tokio::test]
async fn returns_invalid_file_when_upload_is_neither_a_3mf_nor_json() {
    let (status, body) = post_multipart("/api/profiles/import-preset", "file", "x.3mf", b"not a zip").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_file");
}

#[tokio::test]
async fn returns_bad_request_when_no_file_field_is_present() {
    let (status, body) = post_multipart("/api/profiles/import-preset", "not_file", "x.3mf", b"junk").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "No file provided.");
}

// github.com/t2vi/spoolbook/issues/99 -- ARCHIVE_URL-style env override, same local-mock-server
// pattern as tests/reslicing.rs/tests/google_oauth.rs. Serves a tiny fake BBL.json + one filament
// preset with a 2-level inherits chain (leaf -> mid -> base), so resolution is actually exercised
// end to end, not just a single-level lookup.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn spawn_mock_slicer_service() -> String {
    use axum::Json;
    use axum::extract::Path;
    use tokio::net::TcpListener;

    async fn manifest() -> Json<Value> {
        Json(serde_json::json!({
            "filament_list": [
                { "name": "Bambu PLA Basic @BBL X1C", "sub_path": "filament/Bambu PLA Basic @BBL X1C.json" },
                { "name": "Bambu PLA Basic @base", "sub_path": "filament/Bambu PLA Basic @base.json" },
                { "name": "fdm_filament_pla", "sub_path": "filament/fdm_filament_pla.json" }
            ]
        }))
    }

    async fn preset(Path(name): Path<String>) -> Json<Value> {
        Json(match name.as_str() {
            "Bambu PLA Basic @BBL X1C.json" => serde_json::json!({
                "name": "Bambu PLA Basic @BBL X1C",
                "inherits": "Bambu PLA Basic @base",
                "nozzle_temperature": ["220"]
            }),
            "Bambu PLA Basic @base.json" => serde_json::json!({
                "name": "Bambu PLA Basic @base",
                "inherits": "fdm_filament_pla",
                "hot_plate_temp": ["55"]
            }),
            _ => serde_json::json!({ "name": "fdm_filament_pla", "filament_soluble": ["0"] }),
        })
    }

    let app = axum::Router::new()
        .route("/profiles/BBL.json", axum::routing::get(manifest))
        .route("/profiles/BBL/filament/{name}", axum::routing::get(preset));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

async fn get_json(pool: &sqlx::SqlitePool, uri: &str, cookie: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let response = spoolbook_rs::app(pool.clone()).oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

async fn post_json(pool: &sqlx::SqlitePool, uri: &str, cookie: &str, body: Value) -> (StatusCode, Value) {
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

#[tokio::test]
async fn system_presets_lists_names_from_the_slicer_services_bundled_manifest() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("RESLICE_SERVICE_URL", spawn_mock_slicer_service().await) };
    let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let (status, body) = get_json(&pool, "/api/profiles/system-presets", None).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let names = body["names"].as_array().unwrap();
    assert!(names.contains(&Value::String("Bambu PLA Basic @BBL X1C".to_string())), "{names:?}");
}

#[tokio::test]
async fn resolving_a_system_preset_walks_the_full_inherits_chain() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("RESLICE_SERVICE_URL", spawn_mock_slicer_service().await) };
    let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    let cookie = common::auth_cookie_header(&pool).await;

    let (status, body) =
        post_json(&pool, "/api/profiles/system-presets/resolve", &cookie, serde_json::json!({ "name": "Bambu PLA Basic @BBL X1C" })).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["fields"]["NozzleTempC"], "220", "leaf's own value should win");
    assert_eq!(body["fields"]["HotPlateTempC"], "55", "inherited from the mid-chain parent");
    assert_eq!(body["fields"]["Soluble"], "false", "inherited from the base ancestor two levels up");
}

#[tokio::test]
async fn resolving_an_unknown_system_preset_name_fails_cleanly() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("RESLICE_SERVICE_URL", spawn_mock_slicer_service().await) };
    let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    let cookie = common::auth_cookie_header(&pool).await;

    let (status, body) = post_json(&pool, "/api/profiles/system-presets/resolve", &cookie, serde_json::json!({ "name": "nonexistent" })).await;
    assert_ne!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn importing_a_raw_preset_json_resolves_its_inherits_chain() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("RESLICE_SERVICE_URL", spawn_mock_slicer_service().await) };

    // A user's own exported preset -- its own values win, "hot_plate_temp" isn't set here so it
    // should fall through to the mocked chain's mid-ancestor.
    let own_preset = serde_json::json!({
        "name": "My Cold Day PLA",
        "inherits": "Bambu PLA Basic @base",
        "nozzle_temperature": ["230"]
    });

    let (status, body) = post_multipart("/api/profiles/import-preset", "file", "My Cold Day PLA.json", own_preset.to_string().as_bytes()).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["suggestedName"], "My Cold Day PLA", "own JSON's name field wins over the filename");
    assert_eq!(body["fields"]["NozzleTempC"], "230", "uploaded file's own value wins");
    assert_eq!(body["fields"]["HotPlateTempC"], "55", "inherited from the resolved chain");
}
