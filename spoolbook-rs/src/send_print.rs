// Port of PrinterPrintService.cs — sends an already-sliced .3mf to the printer and starts it.
// Two steps, both undocumented Bambu LAN-mode protocol: FTPS upload of the file (see
// upload_via_ftps — the one piece of this slice still unverified against real hardware, see its
// own doc comment), then a "project_file" command on the same live MQTT connection
// printer_mqtt's control commands reuse.
use crate::printer_mqtt::LiveStatusStore;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json, Router, routing::post};
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::io::Read;
use std::sync::Arc;
use suppaftp::tokio::{AsyncRustlsConnector, AsyncRustlsFtpStream};

// Web uploads are stored on disk under a content-hash filename (project_upload.rs) — that's fine
// for spoolbook's own storage, but sending it to the printer as the on-device filename got
// rejected ("Unsupported file path or name", Bambu error 0500-4002 000314, confirmed against a
// real P2S). Use the human display name instead, stripped to a conservative safe charset since
// the exact rule the printer's firmware enforces isn't documented.
pub fn sanitize_for_printer_filename(display_file_name: &str) -> String {
    let stem = std::path::Path::new(display_file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let mut safe: String = stem.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
    if safe.is_empty() {
        safe = "print".to_string();
    }
    safe.truncate(60);
    format!("{safe}.3mf")
}

// Confirmed against a real .3mf export: the printer validates this against the plate's gcode
// specifically (matching the archive's own Metadata/plate_N.gcode.md5 sidecar), not the whole
// .3mf file's checksum — whole-file md5 got "unable to parse the 3mf file" (0502-402D) even with
// a correctly gcode-embedded export.
pub fn compute_gcode_md5(local_file_path: &str, plate_gcode_file_name: &str) -> Option<String> {
    let file = std::fs::File::open(local_file_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name(&format!("Metadata/{plate_gcode_file_name}")).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(format!("{:x}", md5::compute(&buf)))
}

// Mirrors maziggy/bambuddy's start_print (backend/app/services/bambu_mqtt.py) field-for-field —
// a mature, production, issue-tracked implementation that special-cases exactly this hardware
// (P2S/N7). Matching OpenBambuAPI-doc-level guessing repeatedly got a command accepted (err_code
// 0) but still failing later at [0502-402D] once AMS validation passed.
pub fn build_project_file_payload(
    remote_file_name: &str,
    md5: &str,
    plate_gcode_file_name: &str,
    use_ams: bool,
    ams_slot: i64,
    is_p2s: bool,
    submission_id: &str,
) -> String {
    let flat_ams_mapping: Vec<i64> = if use_ams { vec![ams_slot] } else { vec![] };
    // Global tray ID = ams_id*4 + slot_id (bambuddy's "regular AMS tray" case — spoolbook only
    // targets a single onboard AMS unit, not AMS-HT/external-spool/multi-nozzle setups).
    let ams_mapping2: Vec<serde_json::Value> =
        flat_ams_mapping.iter().map(|t| json!({ "ams_id": t / 4, "slot_id": t % 4 })).collect();
    let subtask_name = std::path::Path::new(remote_file_name).file_stem().and_then(|s| s.to_str()).unwrap_or(remote_file_name);

    json!({
        "print": {
            "sequence_id": "20000",
            "command": "project_file",
            "param": format!("Metadata/{plate_gcode_file_name}"),
            "url": format!("ftp:///{remote_file_name}"),
            "file": remote_file_name,
            "md5": md5,
            "bed_type": "auto",
            "timelapse": false,
            // bed_leveling stays a plain bool (true only when forced "on"); auto_bed_leveling
            // carries the tri-state (0=off/1=on/2=auto) — the two-field shape Bambu Studio
            // actually sends. Spoolbook always requests "auto".
            "bed_leveling": false,
            "auto_bed_leveling": 2,
            "flow_cali": false,
            // P2S doesn't support vibration calibration like X1/P1 — bambuddy forces this off
            // specifically for P2S/N7.
            "vibration_cali": !is_p2s,
            "layer_inspect": false,
            "use_ams": use_ams,
            "cfg": "0",
            "extrude_cali_flag": 2,
            "extrude_cali_manual_mode": 0,
            // Single-nozzle only (spoolbook has no dual-nozzle printer support) — always 0.
            "nozzle_offset_cali": 0,
            "subtask_name": subtask_name,
            "profile_id": "0",
            // A fresh non-zero id per submission, not hardcoded "0" — hardcoded 0 makes
            // third-party MQTT observers see reprints as a continuation of the same job.
            "project_id": submission_id,
            "subtask_id": submission_id,
            "task_id": submission_id,
            "ams_mapping": flat_ams_mapping,
            "ams_mapping2": ams_mapping2,
        }
    })
    .to_string()
}

pub fn submission_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let id = (millis % 2_147_483_647) as i64;
    if id == 0 { "1".to_string() } else { id.to_string() }
}

fn ftps_tls_config() -> rustls::ClientConfig {
    rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS12])
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(crate::printer_mqtt::NoCertVerification))
        .with_no_client_auth()
}

// FTPS upload to the printer's onboard vsftpd (port 990, implicit TLS, bblp/access-code creds —
// same as MQTT). UNVERIFIED AGAINST REAL HARDWARE, same posture as printer_mqtt.rs. The three
// workarounds the C# original needed (via FluentFTP.GnuTLS, since .NET's SslStream can't do any
// of this) map onto rustls as: a single ClientConfig — and therefore one resumption cache —
// reused across the control and data connections by suppaftp's tokio-rustls backend, ALPN left
// off (rustls's default — never set), and TLS 1.2 forced via builder_with_protocol_versions
// above (TLS 1.3's async session-ticket delivery is what broke resumption timing in the C#
// version). Whether rustls's automatic resumption actually satisfies this specific old vsftpd
// build's require_ssl_reuse check, and whether the data connection opens fast enough relative to
// the control handshake, is the one thing only a real P2S can confirm.
pub async fn upload_via_ftps(ip_address: &str, access_code: &str, local_file_path: &str, remote_file_name: &str) -> Result<(), String> {
    let connector = tokio_rustls::TlsConnector::from(Arc::new(ftps_tls_config()));
    let mut ftp = AsyncRustlsFtpStream::connect_secure_implicit((ip_address, 990), AsyncRustlsConnector::from(connector), ip_address)
        .await
        .map_err(|e| e.to_string())?;

    ftp.login("bblp", access_code).await.map_err(|e| e.to_string())?;

    let mut file = tokio::fs::File::open(local_file_path).await.map_err(|e| e.to_string())?;
    ftp.put_file(format!("/{remote_file_name}"), &mut file).await.map_err(|e| e.to_string())?;
    ftp.quit().await.ok();
    Ok(())
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/api/printers/{id}/print", post(start_print))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartPrintRequest {
    project_id: i64,
    plater_id: String,
    spool_id: i64,
    profile_id: i64,
    use_ams: bool,
    ams_slot: i64,
}

fn err_response(message: &str) -> serde_json::Value {
    json!({ "ok": false, "error": message })
}

async fn start_print(
    State(pool): State<SqlitePool>,
    Extension(store): Extension<LiveStatusStore>,
    Path(id): Path<i64>,
    Json(req): Json<StartPrintRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let Some(printer) = crate::printers::get_by_id(&pool, id).await else {
        return (StatusCode::BAD_REQUEST, Json(err_response("Printer is missing connection details.")));
    };
    let (Some(ip_address), Some(access_code), Some(serial_number)) = (&printer.ip_address, &printer.access_code, &printer.serial_number) else {
        return (StatusCode::BAD_REQUEST, Json(err_response("Printer is missing connection details.")));
    };

    let project = sqlx::query_as::<_, (String, String)>("SELECT file_path, file_name FROM projects WHERE id = ?1")
        .bind(req.project_id)
        .fetch_optional(&pool)
        .await
        .expect("query failed");
    let Some((file_path, file_name)) = project else {
        return (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" })));
    };

    let remote_file_name = sanitize_for_printer_filename(&file_name);
    let plate_gcode_file_name = format!("plate_{}.gcode", req.plater_id);

    let Some(md5) = compute_gcode_md5(&file_path, &plate_gcode_file_name) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(err_response(&format!("Couldn't find Metadata/{plate_gcode_file_name} inside the .3mf — is this a sliced export, not just a saved project?"))),
        );
    };

    if let Err(e) = upload_via_ftps(ip_address, access_code, &file_path, &remote_file_name).await {
        return (StatusCode::BAD_REQUEST, Json(err_response(&format!("File upload to printer failed: {e}"))));
    }

    let is_p2s = printer.model.as_deref().unwrap_or_default().to_lowercase().contains("p2s");
    let payload = build_project_file_payload(&remote_file_name, &md5, &plate_gcode_file_name, req.use_ams, req.ams_slot, is_p2s, &submission_id());

    if let Err(e) = crate::printer_mqtt::publish_raw(&store, id, serial_number, payload).await {
        return (StatusCode::BAD_REQUEST, Json(err_response(&e)));
    }

    crate::prints::create_in_progress(
        &pool,
        req.profile_id,
        req.spool_id,
        id,
        Some(req.project_id),
        Some(&req.plater_id),
        &chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    )
    .await;

    (StatusCode::OK, Json(json!({ "ok": true })))
}
