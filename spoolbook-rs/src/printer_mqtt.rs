use crate::bambu_mqtt_payload_parser as parser;
use crate::printer_telemetry;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterLiveStatus {
    pub connected: bool,
    pub ams_units: Vec<parser::AmsUnitReading>,
    pub gcode_state: Option<String>,
    pub chamber_temp_c: Option<f64>,
    // What the printer is currently complaining about (HMS / print_error), decoded for the UI.
    // Replaced wholesale on every full status, so it clears itself when the printer stops
    // reporting the error.
    pub errors: Vec<parser::PrinterError>,
    // Reused by printer control commands (docs/adr/0022) so pause/resume/stop publish on the
    // same live connection telemetry already holds open, rather than opening a new one per
    // action. AsyncClient is a cheap handle (channel sender), not the real socket — cloning it
    // for the SSE snapshot every 2s is fine.
    #[serde(skip)]
    pub client: Option<AsyncClient>,
}

pub type LiveStatusStore = Arc<RwLock<HashMap<i64, PrinterLiveStatus>>>;

pub fn new_store() -> LiveStatusStore {
    Arc::new(RwLock::new(HashMap::new()))
}

// Abort handles for the per-printer connect_and_subscribe_loop tasks, keyed by printer id.
// spawn_all fills this at startup; printers.rs create/update/delete call respawn_one/stop_one so a
// printer configured while the process is already running gets a live connection without a
// restart. That's the normal case under Docker (fresh data volume, printer added through the UI,
// container never bounced): before this, the one-shot Test Connection worked but the persistent
// loop never started, so the card stayed "Not connected" and Print failed at publish_raw with
// "Printer isn't connected".
pub type ConnSupervisor = Arc<tokio::sync::Mutex<HashMap<i64, tokio::task::AbortHandle>>>;

pub fn new_supervisor() -> ConnSupervisor {
    Arc::new(tokio::sync::Mutex::new(HashMap::new()))
}

// Spawn (or replace) the live-telemetry loop for one printer. Aborts any existing loop for this id
// first, so an edit that changes the IP doesn't leave the old loop reconnecting to the old
// address forever with both loops fighting over store[id]. Collapses to just the abort when the
// printer has no connection details.
#[allow(clippy::too_many_arguments)]
pub async fn respawn_one(
    supervisor: &ConnSupervisor,
    id: i64,
    ip_address: Option<String>,
    access_code: Option<String>,
    serial_number: Option<String>,
    pool: SqlitePool,
    store: LiveStatusStore,
    camera_registry: crate::printer_camera::CameraRegistry,
) {
    let mut sup = supervisor.lock().await;
    if let Some(handle) = sup.remove(&id) {
        handle.abort();
    }
    store.write().await.remove(&id);
    if let (Some(ip_address), Some(access_code), Some(serial_number)) = (ip_address, access_code, serial_number) {
        let task = tokio::spawn(connect_and_subscribe_loop(
            id, ip_address, access_code, serial_number, pool, store, camera_registry,
        ));
        sup.insert(id, task.abort_handle());
    }
}

// Drop the live-telemetry loop for a deleted printer.
pub async fn stop_one(supervisor: &ConnSupervisor, store: &LiveStatusStore, id: i64) {
    if let Some(handle) = supervisor.lock().await.remove(&id) {
        handle.abort();
    }
    store.write().await.remove(&id);
}

pub async fn snapshot(store: &LiveStatusStore, printer_id: i64) -> PrinterLiveStatus {
    store.read().await.get(&printer_id).cloned().unwrap_or_default()
}

// Accepts whatever certificate the printer's LAN broker presents — Bambu's firmware uses a
// self-signed cert, and this is a local-network-only credential exchange (docs/adr/0017), not a
// certificate-authenticated one. Mirrors MQTTnet's `.WithCertificateValidationHandler(_ => true)`.
// Reused by send_print.rs's FTPS upload — same self-signed-cert, LAN-only posture.
#[derive(Debug)]
pub(crate) struct NoCertVerification;

impl rustls::client::danger::ServerCertVerifier for NoCertVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::CryptoProvider::get_default()
            .map(|p| p.signature_verification_algorithms.supported_schemes())
            .unwrap_or_default()
    }
}

fn tls_config() -> rustls::ClientConfig {
    rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertVerification))
        .with_no_client_auth()
}

// Shared by send_print.rs's FTPS upload and printer_camera.rs's RTSP TLS proxy — both need TLS
// 1.2 forced (this printer family's firmware has session-handling quirks with 1.3, confirmed in
// both the print-start FTPS fix and the camera proxy's own doc comments) on top of the same
// accept-any-cert posture as tls_config() above.
pub(crate) fn tls12_no_verify_config() -> rustls::ClientConfig {
    rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS12])
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertVerification))
        .with_no_client_auth()
}

// Auto-connects at launch to every configured Printer's local MQTT broker (docs/adr/0017) and
// buffers live telemetry via printer_telemetry. Read-only: subscribes to device/{serial}/report
// only, never publishes to a printer's request/control topic.
//
// Confirmed against a real Bambu P2S: the connection itself ConnAcks/SubAcks fine, but rumqttc's
// default 10KB max packet size is smaller than this printer's real device/report payload
// (~14KB) — connect_and_subscribe_loop's own set_max_packet_size call below is the fix. No
// reference test suite to port forward (the .NET original has none either).
pub async fn spawn_all(pool: SqlitePool, store: LiveStatusStore, camera_registry: crate::printer_camera::CameraRegistry, supervisor: ConnSupervisor) {
    tokio::spawn(purge_stale_jobs_loop(pool.clone()));

    let printers = sqlx::query_as::<_, (i64, Option<String>, Option<String>, Option<String>)>(
        "SELECT id, ip_address, access_code, serial_number FROM printers",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    for (id, ip_address, access_code, serial_number) in printers {
        respawn_one(&supervisor, id, ip_address, access_code, serial_number, pool.clone(), store.clone(), camera_registry.clone()).await;
    }
}

// ADR-0017's 7-day unattached-Job retention. Runs once at startup, then daily.
async fn purge_stale_jobs_loop(pool: SqlitePool) {
    loop {
        sqlx::query("DELETE FROM printer_jobs WHERE print_id IS NULL AND started_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-7 days')")
            .execute(&pool)
            .await
            .ok();
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
    }
}

// rumqttc has no MQTTnet-style "connect once, receive via a separate event callback" split —
// polling the eventloop is simultaneously what maintains the connection and what delivers
// messages. So unlike the .NET original's connect-then-watchdog-loop shape, this is one loop
// that does both; behaviorally equivalent (connected, receiving, reconnecting on failure).
// pub rather than private: driven directly by tests/printer_live.rs against a real printer on
// the LAN (no mock -- the bugs this catches are all "the firmware's own wire format / packet
// size", invisible to anything but real hardware). Same testing boundary as handle_message.
pub async fn connect_and_subscribe_loop(
    printer_id: i64,
    ip_address: String,
    access_code: String,
    serial_number: String,
    pool: SqlitePool,
    store: LiveStatusStore,
    camera_registry: crate::printer_camera::CameraRegistry,
) {
    // Tracks the external job id of the currently-active print, so a terminal message (which
    // may omit task_id) can still end the right Job. Mirrors _activeTaskIdByPrinter — lost on
    // restart, same as the .NET original.
    let mut active_task_id: Option<String> = None;

    loop {
        let mut mqttoptions = MqttOptions::new(format!("spoolbook-{}", uuid_v4()), ip_address.clone(), 8883);
        mqttoptions.set_credentials("bblp", &access_code);
        mqttoptions.set_keep_alive(Duration::from_secs(30));
        mqttoptions.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(Arc::new(tls_config()))));
        // rumqttc's default incoming limit (10KB) is smaller than this printer's real device/report
        // payload (~14KB delta, confirmed against a real P2S -- the connection ConnAcks and SubAcks
        // fine, then errors out the instant the first real status report arrives). The pushall
        // *full* object (every AMS tray + the whole HMS array) is bigger again, so 128KB wasn't
        // always enough -- an oversized-packet poll error right after ConnAck is exactly the
        // "connects, flips to Live, then drops to No live job data" flap. 1MB is well clear.
        mqttoptions.set_max_packet_size(1024 * 1024, 1024 * 1024);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        let topic = format!("device/{serial_number}/report");
        if client.subscribe(&topic, QoS::AtMostOnce).await.is_err() {
            eprintln!("[mqtt {printer_id}] subscribe to {topic} failed, retrying in 15s");
            tokio::time::sleep(Duration::from_secs(15)).await;
            continue;
        }
        eprintln!("[mqtt {printer_id}] connecting to {ip_address}:8883");
        store.write().await.entry(printer_id).or_default().client = Some(client.clone());

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    eprintln!("[mqtt {printer_id}] connected, sending pushall");
                    store.write().await.entry(printer_id).or_default().connected = true;
                    // Bambu's broker only emits a full status object (gcode_state, task_id, ams,
                    // the pause/HMS reason) on request or on its own slow (~minutes) schedule --
                    // deltas in between omit gcode_state and parser::parse drops them. Without
                    // this, a connection made mid-print (the normal case now that telemetry
                    // starts when a printer is added at runtime) shows "No live job data yet"
                    // until the next scheduled full push.
                    let request_topic = format!("device/{serial_number}/request");
                    let _ = client
                        .publish(request_topic, QoS::AtMostOnce, false, r#"{"pushing":{"sequence_id":"0","command":"pushall"}}"#)
                        .await;
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    if let Ok(payload) = std::str::from_utf8(&publish.payload) {
                        // Set SPOOLBOOK_MQTT_DEBUG=1 to dump every raw report to stderr -- the
                        // only way to see the printer's own pause/HMS reason, gcode_state, and
                        // error codes, none of which the parser keeps. Off by default (a real
                        // report is ~14KB).
                        if std::env::var("SPOOLBOOK_MQTT_DEBUG").is_ok() {
                            eprintln!("[mqtt {printer_id}] report: {payload}");
                        }
                        handle_message(printer_id, payload, &pool, &store, &mut active_task_id, &camera_registry).await;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[mqtt {printer_id}] disconnected: {e} -- reconnecting in 15s");
                    break;
                }
            }
        }

        store.write().await.remove(&printer_id);
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

// pub rather than private: exercised directly by tests (no real MQTT connection needed, it's
// pure JSON-in/DB-and-store-out), same testing boundary as printer_telemetry.rs.
pub async fn handle_message(
    printer_id: i64,
    payload: &str,
    pool: &SqlitePool,
    store: &LiveStatusStore,
    active_task_id: &mut Option<String>,
    camera_registry: &crate::printer_camera::CameraRegistry,
) {
    let Some(message) = parser::parse(payload) else { return };

    {
        let mut s = store.write().await;
        let entry = s.entry(printer_id).or_default();
        entry.connected = true;
        entry.gcode_state = Some(message.gcode_state.clone());
        // Every message that carries gcode_state also carries the current error state, so this
        // is a full replace: a cleared error disappears from the card on its own.
        entry.errors = message.errors.clone();
        if !message.ams_units.is_empty() {
            entry.ams_units = message.ams_units.clone();
        }
        // A delta message that omits chamber_temper must not clobber the last known reading,
        // same reasoning as the ams_units guard above.
        if message.reading.chamber_temp_c.is_some() {
            entry.chamber_temp_c = message.reading.chamber_temp_c;
        }
    }

    if parser::is_active_state(&message.gcode_state) {
        if let Some(task_id) = &message.task_id {
            *active_task_id = Some(task_id.clone());
            printer_telemetry::record_reading(pool, printer_id, task_id, &message.reading, None).await;
        }
    } else if let Some(task_id) = active_task_id.take() {
        let (chamber_temp_c, ams_humidity_pct) = snapshot_for_end_job(store, printer_id).await;
        printer_telemetry::end_job(pool, printer_id, &task_id, Some(&message.gcode_state), None, chamber_temp_c, ams_humidity_pct, camera_registry).await;
    } else if let Some(task_id) = printer_telemetry::find_open_job_external_id(pool, printer_id).await {
        // active_task_id is only ever in-memory (comment above connect_and_subscribe_loop's
        // declaration) — a backend restart mid-print loses it, but printer_jobs' own open row
        // (ended_at IS NULL) survives. Without this fallback a terminal message arriving after a
        // restart is a silent no-op forever: the Print never gets ended_at/status, stuck
        // InProgress until someone notices and fixes it by hand.
        let (chamber_temp_c, ams_humidity_pct) = snapshot_for_end_job(store, printer_id).await;
        printer_telemetry::end_job(pool, printer_id, &task_id, Some(&message.gcode_state), None, chamber_temp_c, ams_humidity_pct, camera_registry).await;
    }
}

// Best-known chamber temp / AMS humidity for this printer at the moment a print ends — the only
// point either value gets persisted onto the Print row (see prints.rs's comment on PrintRow).
// Takes the first AMS unit's reading when multiple are present; ponytail: revisit if a multi-AMS
// setup makes that ambiguous in practice.
//
// humidity_pct only, never humidity_level: the print-detail UI shows this single stored number
// as a plain percentage, and a coarse 1-5 index displayed that way ("3%") would read as a real,
// very-low humidity reading instead of "middling" on the coarse scale. AMS hardware without a
// hygrometer (no humidity_pct) just gets left blank here rather than showing a wrong number.
async fn snapshot_for_end_job(store: &LiveStatusStore, printer_id: i64) -> (Option<f64>, Option<i64>) {
    let s = store.read().await;
    let Some(entry) = s.get(&printer_id) else { return (None, None) };
    let ams_humidity_pct = entry.ams_units.first().and_then(|u| u.humidity_pct);
    (entry.chamber_temp_c, ams_humidity_pct)
}

// Publishes on the same live connection telemetry already holds open (docs/adr/0022) rather
// than opening a new one per action — so a command fails outright (rather than queuing) if that
// connection happens to be mid-reconnect.
pub async fn publish_command(store: &LiveStatusStore, printer_id: i64, serial_number: &str, command: &str) -> Result<(), String> {
    // sequence_id must be a stringified integer, not a uuid: the P2S firmware ignores pause/
    // resume/stop commands whose sequence_id isn't numeric (the start-print "project_file" path
    // already uses a numeric one via submission_id() and works). This is what makes the on-screen
    // "select Resume to retry" prompt after an HMS actually respond to spoolbook's Resume button.
    let payload = serde_json::json!({ "print": { "command": command, "param": "", "sequence_id": crate::send_print::submission_id() } }).to_string();
    publish_raw(store, printer_id, serial_number, payload).await
}

// Shared by publish_command above and send_print.rs's "project_file" start-print command —
// both just publish an already-built payload on the same live connection telemetry holds open.
pub async fn publish_raw(store: &LiveStatusStore, printer_id: i64, serial_number: &str, payload: String) -> Result<(), String> {
    let client = store.read().await.get(&printer_id).and_then(|s| s.client.clone());
    let Some(client) = client else {
        return Err("Printer isn't connected — telemetry link is down or still reconnecting.".to_string());
    };

    let topic = format!("device/{serial_number}/request");
    client.publish(topic, QoS::AtMostOnce, false, payload).await.map_err(|e| e.to_string())
}

// One-shot diagnostic: connects and disconnects immediately, no subscribe. A successful CONNACK
// already proves both the IP is reachable and the access code is correct — Bambu's broker
// rejects auth at connect time — so there's nothing a subscribe would add here.
pub async fn test_connection(ip_address: &str, access_code: &str) -> Result<(), String> {
    let mut mqttoptions = MqttOptions::new(format!("spoolbook-test-{}", uuid_v4()), ip_address, 8883);
    mqttoptions.set_credentials("bblp", access_code);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(Arc::new(tls_config()))));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let outcome = tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                    return if ack.code == rumqttc::ConnectReturnCode::Success {
                        Ok(())
                    } else {
                        Err(format!("{:?}", ack.code))
                    };
                }
                Ok(_) => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
    })
    .await
    .unwrap_or_else(|_| Err("Timed out — check the IP address and that LAN mode is enabled on the printer.".to_string()));

    client.disconnect().await.ok();
    outcome
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let pid = std::process::id() as u128;
    format!("{:032x}", nanos ^ (pid << 96))
}
