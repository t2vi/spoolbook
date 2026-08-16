use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::io::Read;

// Domain-only slice of ProjectService: list, plate reading, and version-linking. Upload,
// import-from-url, and reslicing are Web-specific integrations (multipart handling, network
// fetch, the separate slicer-service subsystem) — same "not domain layer" boundary as the MQTT/
// camera code, deliberately out of scope here.
#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub last_known_write_time_utc: String,
    pub last_known_file_size_bytes: i64,
    pub mesh_hash: Option<String>,
    pub previous_version_project_id: Option<i64>,
    pub version_number: i64,
    pub is_current_version: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPlate {
    pub plater_id: String,
    pub plater_name: Option<String>,
    pub thumbnail_bytes: Option<String>,
}

const COLUMNS: &str = "id, file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes,
    mesh_hash, previous_version_project_id, version_number, is_current_version";

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/projects", get(list))
        .route("/api/projects/{id}/plates", get(plates))
        .route("/api/projects/version-candidate", get(version_candidate))
        .route("/api/projects/{id}/link-version", axum::routing::post(link_version))
}

async fn list(State(pool): State<SqlitePool>) -> Json<Vec<Project>> {
    let sql = format!("SELECT {COLUMNS} FROM projects ORDER BY file_name");
    let projects = sqlx::query_as::<_, Project>(&sql).fetch_all(&pool).await.expect("query failed");
    Json(projects)
}

// Real zip + XML parsing, matching ProjectService.ReadPlates exactly — reads fresh from disk
// on every call (no cache), same as .NET's stat-based (not content-hashed) drift detection.
fn read_plates(file_path: &str) -> Vec<ProjectPlate> {
    let file = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return vec![],
    };

    let config_xml = {
        let mut entry = match archive.by_name("Metadata/model_settings.config") {
            Ok(e) => e,
            Err(_) => return vec![],
        };
        let mut buf = String::new();
        if entry.read_to_string(&mut buf).is_err() {
            return vec![];
        }
        buf
    };

    let doc = match roxmltree::Document::parse(&config_xml) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut plates = Vec::new();
    for plate_el in doc.root_element().children().filter(|n| n.has_tag_name("plate")) {
        let meta = |key: &str| -> Option<String> {
            plate_el
                .children()
                .filter(|n| n.has_tag_name("metadata"))
                .find(|n| n.attribute("key") == Some(key))
                .and_then(|n| n.attribute("value"))
                .map(str::to_string)
        };

        let Some(plater_id) = meta("plater_id") else { continue };
        let plater_name = meta("plater_name").filter(|s| !s.is_empty());

        let thumbnail_bytes = meta("thumbnail_file").and_then(|thumbnail_file| {
            let mut entry = archive.by_name(&thumbnail_file).ok()?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).ok()?;
            Some(base64::engine::general_purpose::STANDARD.encode(buf))
        });

        plates.push(ProjectPlate { plater_id, plater_name, thumbnail_bytes });
    }

    plates
}

async fn plates(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> Result<Json<Vec<ProjectPlate>>, StatusCode> {
    let file_path = sqlx::query_scalar::<_, String>("SELECT file_path FROM projects WHERE id = ?1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .expect("query failed");

    match file_path {
        Some(file_path) => Ok(Json(read_plates(&file_path))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
struct VersionCandidateQuery {
    #[serde(rename = "meshHash")]
    mesh_hash: Option<String>,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "excludeProjectId")]
    exclude_project_id: Option<i64>,
}

async fn version_candidate(
    State(pool): State<SqlitePool>,
    Query(q): Query<VersionCandidateQuery>,
) -> Json<Option<Project>> {
    if let Some(mesh_hash) = q.mesh_hash.filter(|h| !h.is_empty()) {
        let sql = format!(
            "SELECT {COLUMNS} FROM projects WHERE mesh_hash = ?1 AND (?2 IS NULL OR id != ?2) LIMIT 1"
        );
        let candidate = sqlx::query_as::<_, Project>(&sql)
            .bind(&mesh_hash)
            .bind(q.exclude_project_id)
            .fetch_optional(&pool)
            .await
            .expect("query failed");
        if candidate.is_some() {
            return Json(candidate);
        }
    }

    let sql = format!("SELECT {COLUMNS} FROM projects WHERE file_name = ?1 AND (?2 IS NULL OR id != ?2) LIMIT 1");
    let candidate = sqlx::query_as::<_, Project>(&sql)
        .bind(&q.file_name)
        .bind(q.exclude_project_id)
        .fetch_optional(&pool)
        .await
        .expect("query failed");

    Json(candidate)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkVersionInput {
    previous_version_project_id: i64,
}

async fn link_version(
    State(pool): State<SqlitePool>,
    Path(new_id): Path<i64>,
    Json(input): Json<LinkVersionInput>,
) -> (StatusCode, Json<serde_json::Value>) {
    let not_found = (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "not_found" })));

    let mut tx = pool.begin().await.expect("begin failed");

    let previous_version_number = sqlx::query_scalar::<_, i64>("SELECT version_number FROM projects WHERE id = ?1")
        .bind(input.previous_version_project_id)
        .fetch_optional(&mut *tx)
        .await
        .expect("query failed");
    let Some(previous_version_number) = previous_version_number else {
        return not_found;
    };

    sqlx::query("UPDATE projects SET is_current_version = 0 WHERE id = ?1")
        .bind(input.previous_version_project_id)
        .execute(&mut *tx)
        .await
        .expect("update failed");

    let updated = sqlx::query(
        "UPDATE projects SET previous_version_project_id = ?1, version_number = ?2, is_current_version = 1
         WHERE id = ?3",
    )
    .bind(input.previous_version_project_id)
    .bind(previous_version_number + 1)
    .bind(new_id)
    .execute(&mut *tx)
    .await
    .expect("update failed");

    if updated.rows_affected() == 0 {
        return not_found;
    }

    tx.commit().await.expect("commit failed");
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}
