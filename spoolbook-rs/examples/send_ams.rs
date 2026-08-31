// Send a project_file with a configurable use_ams / ams_mapping, to find what a cold P2S
// (nothing loaded in the extruder) actually accepts. Uploads over FTPS, publishes, watches for
// err / gcode_state for ~60s, then stops.
//
//   cargo run --example send_ams -- <ip> <code> <serial> <sliced.3mf> <plate.gcode> <use_ams:0|1> <ams_mapping_json>
//   e.g.  ... plate_1.gcode 1 "[1]"      # use AMS, map gcode filament 0 -> global tray 1
//   e.g.  ... plate_1.gcode 0 "[255]"    # external spool
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use std::time::Duration;

#[derive(Debug)]
struct NoCertVerification;
impl rustls::client::danger::ServerCertVerifier for NoCertVerification {
    fn verify_server_cert(&self, _: &rustls::pki_types::CertificateDer<'_>, _: &[rustls::pki_types::CertificateDer<'_>], _: &rustls::pki_types::ServerName<'_>, _: &[u8], _: rustls::pki_types::UnixTime) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>, _: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(&self, _: &[u8], _: &rustls::pki_types::CertificateDer<'_>, _: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::CryptoProvider::get_default().map(|p| p.signature_verification_algorithms.supported_schemes()).unwrap_or_default()
    }
}
fn tls_config() -> rustls::ClientConfig {
    rustls::ClientConfig::builder().dangerous().with_custom_certificate_verifier(std::sync::Arc::new(NoCertVerification)).with_no_client_auth()
}

#[tokio::main]
async fn main() {
    let a: Vec<String> = std::env::args().collect();
    let [_, ip, code, serial, threemf, plate, use_ams, mapping] = a.as_slice() else {
        eprintln!("usage: send_ams <ip> <code> <serial> <sliced.3mf> <plate.gcode> <use_ams:0|1> <ams_mapping_json>");
        std::process::exit(1);
    };
    let use_ams: bool = use_ams == "1";
    let mapping: serde_json::Value = serde_json::from_str(mapping).expect("ams_mapping must be JSON array");

    let md5 = spoolbook_rs::send_print::compute_gcode_md5(threemf, plate).expect("no such plate gcode in .3mf");
    let remote = spoolbook_rs::send_print::sanitize_for_printer_filename("send_ams.3mf");
    spoolbook_rs::send_print::upload_via_ftps(ip, code, threemf, &remote).await.expect("ftps upload");
    println!("uploaded {remote}, md5 {md5}");

    let sub = spoolbook_rs::send_print::submission_id();
    let payload = serde_json::json!({
        "print": {
            "sequence_id": "20000",
            "command": "project_file",
            "param": format!("Metadata/{plate}"),
            "url": format!("ftp:///{remote}"),
            "file": remote,
            "md5": md5,
            "bed_type": "auto",
            "timelapse": false,
            "bed_leveling": false,
            "auto_bed_leveling": 2,
            "flow_cali": false,
            "vibration_cali": false,
            "layer_inspect": false,
            "use_ams": use_ams,
            "cfg": "0",
            "extrude_cali_flag": 2,
            "extrude_cali_manual_mode": 0,
            "nozzle_offset_cali": 0,
            "subtask_name": "send_ams",
            "profile_id": "0",
            "project_id": sub, "subtask_id": sub, "task_id": sub,
            "ams_mapping": mapping,
        }
    }).to_string();
    println!("\n=== payload ===\n{payload}\n");

    let mut o = MqttOptions::new(format!("spoolbook-sendams-{}", std::process::id()), ip.clone(), 8883);
    o.set_credentials("bblp", code);
    o.set_keep_alive(Duration::from_secs(30));
    o.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(std::sync::Arc::new(tls_config()))));
    o.set_max_packet_size(1024 * 1024, 1024 * 1024);
    let (client, mut eventloop) = AsyncClient::new(o, 10);
    client.subscribe(format!("device/{serial}/report"), QoS::AtMostOnce).await.expect("subscribe");
    let req = format!("device/{serial}/request");

    let mut sent = false;
    for _ in 0..150 {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                client.publish(&req, QoS::AtMostOnce, false, payload.clone()).await.expect("publish");
                println!(">>> project_file sent");
                sent = true;
            }
            Ok(Event::Incoming(Packet::Publish(p))) => {
                let b = String::from_utf8_lossy(&p.payload);
                let g = find(&b, "\"gcode_state\"");
                let e = find(&b, "\"err\"");
                let tn = find(&b, "\"tray_now\"");
                if !g.is_empty() || !e.is_empty() {
                    println!("  {g}  {e}  {tn}");
                }
            }
            Ok(_) => {}
            Err(e) => { eprintln!("poll err: {e}"); if sent { break; } }
        }
    }
    let seq = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    client.publish(&req, QoS::AtMostOnce, false, format!(r#"{{"print":{{"command":"stop","param":"","sequence_id":"{seq}"}}}}"#)).await.ok();
    println!("<<< stop sent");
}

fn find<'a>(hay: &'a str, key: &str) -> String {
    match hay.find(key) {
        Some(i) => hay[i..(i + 40).min(hay.len())].replace('\n', " "),
        None => String::new(),
    }
}
