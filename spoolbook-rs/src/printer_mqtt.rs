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
// UNVERIFIED AGAINST REAL HARDWARE — this file has no reference test suite to port forward
// (the .NET original has none either); it can only be validated against a real Bambu P2S.
pub async fn spawn_all(pool: SqlitePool, store: LiveStatusStore) {
    tokio::spawn(purge_stale_jobs_loop(pool.clone()));

    let printers = sqlx::query_as::<_, (i64, Option<String>, Option<String>, Option<String>)>(
        "SELECT id, ip_address, access_code, serial_number FROM printers",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    for (id, ip_address, access_code, serial_number) in printers {
        if let (Some(ip_address), Some(access_code), Some(serial_number)) = (ip_address, access_code, serial_number) {
            tokio::spawn(connect_and_subscribe_loop(id, ip_address, access_code, serial_number, pool.clone(), store.clone()));
        }
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
async fn connect_and_subscribe_loop(
    printer_id: i64,
    ip_address: String,
    access_code: String,
    serial_number: String,
    pool: SqlitePool,
    store: LiveStatusStore,
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

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        let topic = format!("device/{serial_number}/report");
        if client.subscribe(&topic, QoS::AtMostOnce).await.is_err() {
            tokio::time::sleep(Duration::from_secs(15)).await;
            continue;
        }
        store.write().await.entry(printer_id).or_default().client = Some(client.clone());

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    store.write().await.entry(printer_id).or_default().connected = true;
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    if let Ok(payload) = std::str::from_utf8(&publish.payload) {
                        handle_message(printer_id, payload, &pool, &store, &mut active_task_id).await;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        store.write().await.remove(&printer_id);
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

// pub rather than private: exercised directly by tests (no real MQTT connection needed, it's
// pure JSON-in/DB-and-store-out), same testing boundary as printer_telemetry.rs.
pub async fn handle_message(printer_id: i64, payload: &str, pool: &SqlitePool, store: &LiveStatusStore, active_task_id: &mut Option<String>) {
    let Some(message) = parser::parse(payload) else { return };

    {
        let mut s = store.write().await;
        let entry = s.entry(printer_id).or_default();
        entry.connected = true;
        entry.gcode_state = Some(message.gcode_state.clone());
        if !message.ams_units.is_empty() {
            entry.ams_units = message.ams_units.clone();
        }
    }

    if parser::is_active_state(&message.gcode_state) {
        if let Some(task_id) = &message.task_id {
            *active_task_id = Some(task_id.clone());
            printer_telemetry::record_reading(pool, printer_id, task_id, &message.reading, None).await;
        }
    } else if let Some(task_id) = active_task_id.take() {
        printer_telemetry::end_job(pool, printer_id, &task_id, Some(&message.gcode_state), None).await;
    }
}

// Publishes on the same live connection telemetry already holds open (docs/adr/0022) rather
// than opening a new one per action — so a command fails outright (rather than queuing) if that
// connection happens to be mid-reconnect.
pub async fn publish_command(store: &LiveStatusStore, printer_id: i64, serial_number: &str, command: &str) -> Result<(), String> {
    let payload = serde_json::json!({ "print": { "command": command, "sequence_id": uuid_v4() } }).to_string();
    publish_raw(store, printer_id, serial_number, payload).await
}

// Shared by publish_command above and send_print.rs's "project_file" start-print command —
// both just publish an already-built payload on the same live connection telemetry holds open.
pub(crate) async fn publish_raw(store: &LiveStatusStore, printer_id: i64, serial_number: &str, payload: String) -> Result<(), String> {
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
