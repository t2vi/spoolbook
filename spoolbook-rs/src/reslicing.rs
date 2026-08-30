use crate::{profile_config_patcher, profiles, project_upload};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, Router, routing::post};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::io::{Read, Write};
use std::path::PathBuf;

// Port of ReslicingService.cs — patches a Project's .3mf with a PrintProfile's settings
// (profile_config_patcher.rs, the ProfileConfigPatcher.cs port) and posts it to the standalone
// slicer-service (a separate Python/FastAPI deployable that shells out to Bambu Studio — out of
// scope here, same boundary as the C# original never re-implementing that in-process). Base URL
// via RESLICE_SERVICE_URL, defaulting to http://localhost:8100 for local dev.
pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/projects/{id}/reslice", post(reslice))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResliceRequest {
    profile_id: i64,
}

fn err_response(message: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": message })
}

async fn reslice(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(req): Json<ResliceRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let project = sqlx::query_as::<_, (String, String)>("SELECT file_path, file_name FROM projects WHERE id = ?1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .expect("query failed");
    let profile = profiles::get_by_id(&pool, req.profile_id).await;

    let (Some((file_path, file_name)), Some(profile)) = (project, profile) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not_found" })));
    };

    let patched_path = match build_patched_project_file(&file_path, &profile) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(err_response(&format!("Couldn't prepare project for re-slicing: {e}")))),
    };

    let result = slice_and_save(&pool, &patched_path, &file_name, id).await;
    std::fs::remove_file(&patched_path).ok();

    let status = if result["ok"] == true { StatusCode::OK } else { StatusCode::BAD_REQUEST };
    (status, Json(result))
}

// A real slicer's output is never byte-identical run to run (embedded timestamps, re-rendered
// thumbnail, ...), so project_upload's content-hash dedup can never catch "re-sliced the same
// project again" — every call used to insert a brand-new, permanently unlinked project row. Chain
// it as a new version of the project being re-sliced instead (same mechanism PrintForm.svelte's
// manual "looks like a new version — link it?" flow uses, just automatic here since the source
// project is already known exactly, no mesh_hash/file_name matching needed).
async fn slice_and_save(pool: &SqlitePool, patched_path: &std::path::Path, display_name: &str, source_project_id: i64) -> serde_json::Value {
    let sliced_bytes = match slice_via_service(patched_path).await {
        Ok(b) => b,
        Err(e) => return err_response(&format!("Re-slice failed: {e}")),
    };
    let mut result = project_upload::save_bytes(&project_upload::storage_dir(), pool, &sliced_bytes, display_name).await;

    let new_id = result["project"]["id"].as_i64();
    if result["created"] == true && new_id.is_some_and(|new_id| new_id != source_project_id) {
        let new_id = new_id.unwrap();
        if crate::projects::link_version_chain(pool, new_id, source_project_id).await {
            if let Some(project) = crate::projects::get_by_id(pool, new_id).await {
                result["project"] = serde_json::to_value(project).unwrap();
            }
        }
    }

    result
}

// pub for tests/printer_live.rs's non-sliced -> slice -> print flow against the real
// slicer-service container. Takes any .3mf on disk; the caller patches settings first (or not).
pub async fn slice_via_service(patched_path: &std::path::Path) -> Result<Vec<u8>, String> {
    let base_url = std::env::var("RESLICE_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8100".to_string());
    let bytes = std::fs::read(patched_path).map_err(|e| e.to_string())?;
    let file_name = patched_path.file_name().and_then(|n| n.to_str()).unwrap_or("project.3mf").to_string();

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/octet-stream")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new().part("project", part);

    let response = reqwest::Client::new()
        .post(format!("{base_url}/slice"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        return Err(format!("slicer-service returned {}: {detail}", status.as_u16()));
    }

    response.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
}

// Copies the original .3mf (never mutated in place — project_upload's storage is content-hash
// addressed, so the source file is effectively immutable) and swaps its
// Metadata/project_settings.config entry for one patched with the chosen profile's settings.
// Every other entry is raw-copied byte-for-byte (no decompress/recompress).
fn build_patched_project_file(project_file_path: &str, profile: &profiles::PrintProfile) -> Result<PathBuf, String> {
    let source = std::fs::File::open(project_file_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(source).map_err(|e| e.to_string())?;

    let original_json = {
        let mut entry = archive
            .by_name("Metadata/project_settings.config")
            .map_err(|_| "Project has no Metadata/project_settings.config — not a sliced export?".to_string())?;
        let mut s = String::new();
        entry.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };
    let patched_json = profile_config_patcher::patch(&original_json, profile);

    let temp_path = std::env::temp_dir().join(format!("spoolbook-reslice-{}.3mf", unique_suffix()));
    let out = std::fs::File::create(&temp_path).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(out);

    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(|e| e.to_string())?;
        if entry.name() == "Metadata/project_settings.config" {
            continue;
        }
        writer.raw_copy_file(entry).map_err(|e| e.to_string())?;
    }

    writer
        .start_file("Metadata/project_settings.config", zip::write::SimpleFileOptions::default())
        .map_err(|e| e.to_string())?;
    writer.write_all(patched_json.as_bytes()).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;

    Ok(temp_path)
}

fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let pid = std::process::id() as u128;
    format!("{:032x}", nanos ^ (pid << 96))
}
