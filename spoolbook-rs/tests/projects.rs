mod common;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use std::io::Write;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to open in-memory db");
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn send(pool: &sqlx::SqlitePool, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", common::auth_cookie_header())
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

// Builds a minimal real .3mf-shaped zip (not a mock) with one plate + thumbnail, matching what
// ProjectService.ReadPlates actually parses in .NET. Returns the file path.
fn write_fixture_3mf(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("3D/3dmodel.model", options).unwrap();
    zip.write_all(b"<mesh>fake geometry</mesh>").unwrap();

    zip.start_file("Metadata/model_settings.config", options).unwrap();
    zip.write_all(
        br#"<config>
            <plate>
                <metadata key="plater_id" value="1"/>
                <metadata key="plater_name" value="Plate 1"/>
                <metadata key="thumbnail_file" value="Metadata/plate_1.png"/>
            </plate>
        </config>"#,
    )
    .unwrap();

    zip.start_file("Metadata/plate_1.png", options).unwrap();
    zip.write_all(b"\x89PNGfakepngbytes").unwrap();

    zip.finish().unwrap();
    path
}

async fn seed_project(pool: &sqlx::SqlitePool, file_path: &str, mesh_hash: Option<&str>) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO projects (file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes, mesh_hash)
         VALUES (?1, ?2, '2026-01-01T00:00:00Z', 100, ?3)
         RETURNING id",
    )
    .bind(file_path)
    .bind("test.3mf")
    .bind(mesh_hash)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn list_returns_empty_array_when_none_exist() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "GET", "/api/projects", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_returns_seeded_projects() {
    let pool = test_pool().await;
    seed_project(&pool, "/tmp/a.3mf", None).await;

    let (status, body) = send(&pool, "GET", "/api/projects", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["filePath"], "/tmp/a.3mf");
}

#[tokio::test]
async fn plates_reads_real_zip_and_returns_plate_metadata_and_thumbnail() {
    let pool = test_pool().await;
    let fixture_path = write_fixture_3mf("spoolbook_rs_test_projects_plates.3mf");
    let id = seed_project(&pool, fixture_path.to_str().unwrap(), None).await;

    let (status, body) = send(&pool, "GET", &format!("/api/projects/{id}/plates"), None).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let plates = body.as_array().unwrap();
    assert_eq!(plates.len(), 1);
    assert_eq!(plates[0]["platerId"], "1");
    assert_eq!(plates[0]["platerName"], "Plate 1");
    assert!(!plates[0]["thumbnailBytes"].as_str().unwrap().is_empty());

    std::fs::remove_file(fixture_path).ok();
}

#[tokio::test]
async fn plates_returns_not_found_for_missing_project_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "GET", "/api/projects/999/plates", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn version_candidate_matches_by_mesh_hash_first() {
    let pool = test_pool().await;
    seed_project(&pool, "/tmp/old.3mf", Some("hash-abc")).await;
    seed_project(&pool, "/tmp/other.3mf", Some("hash-zzz")).await;

    let (status, body) = send(
        &pool,
        "GET",
        "/api/projects/version-candidate?meshHash=hash-abc&fileName=whatever.3mf",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["filePath"], "/tmp/old.3mf");
}

#[tokio::test]
async fn version_candidate_falls_back_to_filename() {
    let pool = test_pool().await;
    seed_project(&pool, "/tmp/old_name.3mf", None).await;
    sqlx::query("UPDATE projects SET file_name = 'match_me.3mf' WHERE file_path = '/tmp/old_name.3mf'")
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = send(
        &pool,
        "GET",
        "/api/projects/version-candidate?fileName=match_me.3mf",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["fileName"], "match_me.3mf");
}

#[tokio::test]
async fn version_candidate_returns_null_when_nothing_matches() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "GET", "/api/projects/version-candidate?fileName=nope.3mf", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Null);
}

#[tokio::test]
async fn link_version_marks_previous_superseded_and_bumps_version_number() {
    let pool = test_pool().await;
    let old_id = seed_project(&pool, "/tmp/v1.3mf", None).await;
    let new_id = seed_project(&pool, "/tmp/v2.3mf", None).await;

    let (status, body) = send(
        &pool,
        "POST",
        &format!("/api/projects/{new_id}/link-version"),
        Some(serde_json::json!({ "previousVersionProjectId": old_id })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);

    let (_, all) = send(&pool, "GET", "/api/projects", None).await;
    let old = all.as_array().unwrap().iter().find(|p| p["id"] == old_id).unwrap();
    let new = all.as_array().unwrap().iter().find(|p| p["id"] == new_id).unwrap();
    assert_eq!(old["isCurrentVersion"], false);
    assert_eq!(new["isCurrentVersion"], true);
    assert_eq!(new["versionNumber"], 2);
    assert_eq!(new["previousVersionProjectId"], old_id);
}
