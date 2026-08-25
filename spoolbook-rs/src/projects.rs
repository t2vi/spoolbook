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

// Upload and import-from-url (see project_upload.rs) build on the mesh-hash/upsert helpers
// below — plain file I/O and an HTTP GET, no different in kind from filament_catalog_sync's
// fetch. Only reslicing stays out of scope: it shells out to Bambu Studio as an external
// process, the same deferred tier as MQTT/camera (live hardware/process integration, not data
// layer).
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

pub(crate) const COLUMNS: &str = "id, file_path, file_name, last_known_write_time_utc, last_known_file_size_bytes,
    mesh_hash, previous_version_project_id, version_number, is_current_version";

// sha256 of just the mesh geometry entry, not the whole file — a re-slice of the same design
// changes settings/thumbnails but usually not this. Byte-exact, not canonicalized (docs/adr/0023).
// Returns None for anything that isn't a readable .3mf zip with that entry, rather than panicking.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn compute_mesh_hash(file_path: &str) -> Option<String> {
    use sha2::{Digest, Sha256};

    let file = std::fs::File::open(file_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name("3D/3dmodel.model").ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(hex(&Sha256::digest(&buf)))
}

pub(crate) async fn get_by_id(pool: &SqlitePool, id: i64) -> Option<Project> {
    let sql = format!("SELECT {COLUMNS} FROM projects WHERE id = ?1");
    sqlx::query_as::<_, Project>(&sql).bind(id).fetch_optional(pool).await.expect("query failed")
}

pub(crate) struct UpsertResult {
    pub project: Project,
    pub created: bool,
}

// Mirrors ProjectService.UpsertByPathAsync. last_known_write_time_utc is stamped at import
// time via SQLite's own now() rather than the file's OS mtime — nothing in this crate reads
// that column back for drift comparison yet (GetFileStatus isn't ported), so exact OS-mtime
// fidelity isn't worth pulling in a date/time crate for. Revisit if drift detection lands.
pub(crate) async fn upsert_by_path(pool: &SqlitePool, file_path: &str, display_name: &str) -> Option<UpsertResult> {
    let size = std::fs::metadata(file_path).ok()?.len() as i64;

    let existing_id = sqlx::query_scalar::<_, i64>("SELECT id FROM projects WHERE file_path = ?1")
        .bind(file_path)
        .fetch_optional(pool)
        .await
        .expect("query failed");

    let created = existing_id.is_none();
    let id = match existing_id {
        Some(id) => id,
        None => {
            let mesh_hash = compute_mesh_hash(file_path);
            sqlx::query_scalar::<_, i64>(
                "INSERT INTO projects (file_path, file_name, mesh_hash, last_known_write_time_utc, last_known_file_size_bytes)
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?4)
                 RETURNING id",
            )
            .bind(file_path)
            .bind(display_name)
            .bind(&mesh_hash)
            .bind(size)
            .fetch_one(pool)
            .await
            .expect("insert failed")
        }
    };

    if !created {
        sqlx::query(
            "UPDATE projects SET last_known_write_time_utc = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), last_known_file_size_bytes = ?1
             WHERE id = ?2",
        )
        .bind(size)
        .bind(id)
        .execute(pool)
        .await
        .expect("update failed");
    }

    let sql = format!("SELECT {COLUMNS} FROM projects WHERE id = ?1");
    let project = sqlx::query_as::<_, Project>(&sql).bind(id).fetch_one(pool).await.expect("query failed");
    Some(UpsertResult { project, created })
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/projects", get(list))
        .route("/api/projects/{id}/plates", get(plates))
        .route("/api/projects/version-candidate", get(version_candidate))
        .route("/api/projects/{id}/link-version", axum::routing::post(link_version))
        .route("/api/projects/{id}", axum::routing::put(rename).delete(delete))
}

async fn has_prints(pool: &SqlitePool, project_id: i64) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM prints WHERE project_id = ?1")
        .bind(project_id)
        .fetch_one(pool)
        .await
        .expect("query failed")
        > 0
}

#[derive(Deserialize)]
struct RenameInput {
    #[serde(rename = "fileName")]
    file_name: String,
}

async fn rename(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<RenameInput>,
) -> (StatusCode, Json<serde_json::Value>) {
    if input.file_name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": "Name is required" })));
    }

    let result = sqlx::query("UPDATE projects SET file_name = ?1 WHERE id = ?2")
        .bind(input.file_name.trim())
        .bind(id)
        .execute(&pool)
        .await
        .expect("update failed");

    if result.rows_affected() == 0 {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "not_found" })));
    }

    let project = get_by_id(&pool, id).await;
    (StatusCode::OK, Json(serde_json::json!({ "ok": true, "project": project })))
}

async fn delete(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<serde_json::Value>) {
    if has_prints(&pool, id).await {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": "has_prints" })));
    }

    let result = sqlx::query("DELETE FROM projects WHERE id = ?1").bind(id).execute(&pool).await.expect("delete failed");

    if result.rows_affected() == 0 {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "not_found" })));
    }

    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

// Current versions only — every re-slice used to create a permanent, unlinked row (mesh_hash was
// computed but nothing chained on it), so this list piled up with near-duplicates from repeated
// re-slice attempts. reslicing.rs now chains those via link_version_chain, so old versions exist
// only as history (previous_version_project_id), same as PrintProfile's is_current_version.
async fn list(State(pool): State<SqlitePool>) -> Json<Vec<Project>> {
    let sql = format!("SELECT {COLUMNS} FROM projects WHERE is_current_version = 1 ORDER BY file_name");
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
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Path(new_id): Path<i64>,
    Json(input): Json<LinkVersionInput>,
) -> (StatusCode, Json<serde_json::Value>) {
    if link_version_chain(&pool, new_id, input.previous_version_project_id).await {
        (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "not_found" })))
    }
}

// Shared by the manual "looks like a new version — link it?" flow (PrintForm.svelte, matched by
// mesh_hash/file_name since the upload there has no known source project) and reslicing.rs's
// automatic chaining (source project is already known exactly, no matching/confirmation needed).
pub(crate) async fn link_version_chain(pool: &SqlitePool, new_id: i64, previous_version_project_id: i64) -> bool {
    let mut tx = pool.begin().await.expect("begin failed");

    let previous_version_number = sqlx::query_scalar::<_, i64>("SELECT version_number FROM projects WHERE id = ?1")
        .bind(previous_version_project_id)
        .fetch_optional(&mut *tx)
        .await
        .expect("query failed");
    let Some(previous_version_number) = previous_version_number else {
        return false;
    };

    sqlx::query("UPDATE projects SET is_current_version = 0 WHERE id = ?1")
        .bind(previous_version_project_id)
        .execute(&mut *tx)
        .await
        .expect("update failed");

    let updated = sqlx::query(
        "UPDATE projects SET previous_version_project_id = ?1, version_number = ?2, is_current_version = 1
         WHERE id = ?3",
    )
    .bind(previous_version_project_id)
    .bind(previous_version_number + 1)
    .bind(new_id)
    .execute(&mut *tx)
    .await
    .expect("update failed");

    if updated.rows_affected() == 0 {
        return false;
    }

    tx.commit().await.expect("commit failed");
    true
}
