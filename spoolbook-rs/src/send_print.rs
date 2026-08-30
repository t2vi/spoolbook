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
    // Both kept for the wire contract with the frontend's AMS toggle + tray picker, but neither
    // is sent: spoolbook's .3mf is re-sliced by the headless BambuStudio CLI, whose gcode carries
    // no per-filament AMS-assignment metadata. With that missing, use_ams:true (with or without
    // an ams_mapping) fails at HMS 07FF-8012 "Failed to get AMS mapping table" — confirmed
    // against a real P2S: only use_ams:false prints. The printer then just feeds from whatever
    // filament is threaded (the AMS tray's PTFE, in practice). Real AMS/multi-material support
    // needs a file sliced *for* AMS and is a separate feature.
    _use_ams: bool,
    _ams_slot: i64,
    is_p2s: bool,
    submission_id: &str,
) -> String {
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
            // Forced false — see the doc comment on the _use_ams param. A re-sliced spoolbook
            // .3mf has no AMS filament data, so use_ams:true fails at HMS 07FF-8012.
            "use_ams": false,
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
            // Deliberately empty, never a per-tray [tray_id]. spoolbook's .3mf is re-sliced by the
            // headless BambuStudio CLI, whose gcode omits the per-filament AMS-assignment metadata
            // the printer cross-references an incoming mapping against — supplying one (even a
            // correct-looking [ams_id*4+slot_id]) fails at HMS 07FF-8012 "Failed to get AMS
            // mapping table" (confirmed against a real P2S: empty prints fine, [1] does not).
            // Empty tells the printer to feed from its currently-active AMS tray, matching Bambu
            // Handy's own single-colour default. True per-tray selection needs a file sliced *for*
            // AMS and isn't supported here yet.
            "ams_mapping": [],
            "ams_mapping2": [],
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

// FTPS upload to the printer's onboard vsftpd (port 990, implicit TLS, bblp/access-code creds —
// same as MQTT). This printer's vsftpd enforces require_ssl_reuse: the data-channel TLS
// connection must resume the control channel's session, or it's rejected with 522. Two prior
// implementations both failed this exact check against a real P2S: suppaftp+rustls (one shared
// ClientConfig/resumption cache -- "Connection reset by peer"), then curl statically linked
// against vendored OpenSSL (curl-sys's static-ssl -- clean "522 session reuse required", verified
// via verbose logging that the data channel was doing a full handshake, not a resumed one, even
// forced to TLS 1.2). Confirmed root cause empirically: it's specifically the vendored OpenSSL
// build that can't reuse the session here — the identical code linked against the host's system
// libcurl (SecureTransport/LibreSSL on macOS) resumes correctly ("SSL reusing session ID",
// 226 Transfer complete). Same class of bug the C# original hit with .NET's SslStream, fixed
// there by switching to FluentFTP.GnuTLS — a different TLS stack, not a different FTP library,
// same lesson here: curl's `ssl` feature without `static-ssl` links dynamically against the
// platform's own libcurl instead of vendoring OpenSSL from source (see Cargo.toml). Confirmed
// working this way on macOS (dev); the Linux/Docker build still needs its own verification against
// real hardware — Debian's system libcurl is a different OpenSSL build than the one vendored here,
// but "different OpenSSL build" was already true of the two prior failures, so it isn't assumed
// safe without a real test.
// libcurl's `Easy` handle is synchronous; run on a blocking thread rather than pulling in an
// async-curl wrapper for one call site.
pub async fn upload_via_ftps(ip_address: &str, access_code: &str, local_file_path: &str, remote_file_name: &str) -> Result<(), String> {
    let ip_address = ip_address.to_string();
    let access_code = access_code.to_string();
    let local_file_path = local_file_path.to_string();
    let remote_file_name = remote_file_name.to_string();

    tokio::task::spawn_blocking(move || upload_via_ftps_blocking(&ip_address, &access_code, &local_file_path, &remote_file_name))
        .await
        .map_err(|e| e.to_string())?
}

fn upload_via_ftps_blocking(ip_address: &str, access_code: &str, local_file_path: &str, remote_file_name: &str) -> Result<(), String> {
    let mut file = std::fs::File::open(local_file_path).map_err(|e| e.to_string())?;
    let file_len = file.metadata().map_err(|e| e.to_string())?.len();

    let mut easy = curl::easy::Easy::new();
    // ftps:// + an explicit 990 port is curl's implicit-TLS mode (TLS negotiated on connect, no
    // AUTH TLS command) -- matches Bambu's documented LAN-mode protocol, same as the old
    // connect_secure_implicit call this replaces.
    easy.url(&format!("ftps://{ip_address}:990/{remote_file_name}")).map_err(|e| e.to_string())?;
    easy.username("bblp").map_err(|e| e.to_string())?;
    easy.password(access_code).map_err(|e| e.to_string())?;
    // Self-signed on-device cert, same trust posture as printer_mqtt.rs's NoCertVerification.
    easy.ssl_verify_peer(false).map_err(|e| e.to_string())?;
    easy.ssl_verify_host(false).map_err(|e| e.to_string())?;
    easy.upload(true).map_err(|e| e.to_string())?;
    easy.in_filesize(file_len).map_err(|e| e.to_string())?;

    let mut transfer = easy.transfer();
    transfer.read_function(move |buf| Ok(file.read(buf).unwrap_or(0))).map_err(|e| e.to_string())?;
    transfer.perform().map_err(|e| e.to_string())?;
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
    _editor: crate::auth::Editor,
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

    // Set SPOOLBOOK_MQTT_DEBUG=1 to dump the exact project_file command sent to the printer,
    // alongside printer_mqtt's incoming report dump -- the only way to see why a send is
    // rejected (HMS code, gcode_state) on an install we can't reproduce locally.
    if std::env::var("SPOOLBOOK_MQTT_DEBUG").is_ok() {
        eprintln!(
            "[send_print] file={file_path} plate={plate_gcode_file_name} md5={md5} req.use_ams={} req.ams_slot={}\n[send_print] project_file -> {payload}",
            req.use_ams, req.ams_slot
        );
    }

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
