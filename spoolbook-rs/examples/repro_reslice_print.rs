// Reproduce the web "Re-slice with this profile" -> send flow outside the HTTP layer, to see
// why it hits HMS 07FF-8012 when a direct slice-and-send does not.
//
//   cargo run --example repro_reslice_print -- <ip> <access_code> <serial> <project.3mf> <profile_id>
//
// Needs a slicer-service on RESLICE_SERVICE_URL (default http://localhost:8100) and dev.db
// (or SPOOLBOOK_DB_PATH) holding the profile. Prints the exact project_file payload and every
// device/report line for ~40s, then sends `stop`.
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use sqlx::sqlite::SqlitePoolOptions;
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
    let args: Vec<String> = std::env::args().collect();
    let [_, ip, access_code, serial, project_path, profile_id] = args.as_slice() else {
        eprintln!("usage: repro_reslice_print <ip> <access_code> <serial> <project.3mf> <profile_id>");
        std::process::exit(1);
    };
    let profile_id: i64 = profile_id.parse().expect("profile_id must be an int");

    let db_path = std::env::var("SPOOLBOOK_DB_PATH").unwrap_or_else(|_| "dev.db".to_string());
    let pool = SqlitePoolOptions::new().connect(&format!("sqlite://{db_path}")).await.expect("open db");
    let profile = spoolbook_rs::profiles::get_by_id(&pool, profile_id).await.expect("profile not found");
    println!("profile: {}", profile.name);

    // 1. patch project_settings.config with the profile, exactly as /api/projects/{id}/reslice does
    let patched = spoolbook_rs::reslicing::build_patched_project_file(project_path, &profile).expect("patch");
    println!("patched -> {}", patched.display());

    // 2. slice via the real slicer-service
    let sliced = spoolbook_rs::reslicing::slice_via_service(&patched).await.expect("slice");
    std::fs::remove_file(&patched).ok();
    let sliced_path = std::env::temp_dir().join("spoolbook-repro-sliced.3mf");
    std::fs::write(&sliced_path, &sliced).expect("write sliced");
    println!("sliced -> {} ({} bytes)", sliced_path.display(), sliced.len());

    let md5 = spoolbook_rs::send_print::compute_gcode_md5(sliced_path.to_str().unwrap(), "plate_1.gcode")
        .expect("no Metadata/plate_1.gcode in sliced output");
    println!("plate_1.gcode md5: {md5}");

    // 3. upload over FTPS
    let remote = spoolbook_rs::send_print::sanitize_for_printer_filename("repro.3mf");
    spoolbook_rs::send_print::upload_via_ftps(ip, access_code, sliced_path.to_str().unwrap(), &remote).await.expect("ftps upload");
    println!("uploaded as {remote}");

    // 4. build the project_file payload (use_ams:false, forced) and publish it on a live MQTT conn
    let payload = spoolbook_rs::send_print::build_project_file_payload(&remote, &md5, "plate_1.gcode", false, 0, true, &spoolbook_rs::send_print::submission_id());
    println!("\n=== project_file payload ===\n{payload}\n===\n");

    let mut opts = MqttOptions::new(format!("spoolbook-repro-{}", std::process::id()), ip.clone(), 8883);
    opts.set_credentials("bblp", access_code);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(std::sync::Arc::new(tls_config()))));
    opts.set_max_packet_size(1024 * 1024, 1024 * 1024);
    let (client, mut eventloop) = AsyncClient::new(opts, 10);
    client.subscribe(format!("device/{serial}/report"), QoS::AtMostOnce).await.expect("subscribe");
    let request_topic = format!("device/{serial}/request");

    let mut published = false;
    for _ in 0..120 {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                client.publish(&request_topic, QoS::AtMostOnce, false, payload.clone()).await.expect("publish project_file");
                println!(">>> project_file published");
                published = true;
            }
            Ok(Event::Incoming(Packet::Publish(p))) => {
                let body = String::from_utf8_lossy(&p.payload);
                for key in ["gcode_state", "print_error", "\"hms\"", "mc_print_error_code", "fail_reason", "07FF", "8012"] {
                    if let Some(i) = body.find(key) {
                        println!("  {}", &body[i.saturating_sub(2)..(i + 60).min(body.len())].replace('\n', " "));
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("poll error: {e}");
                if published { break; }
            }
        }
    }

    // 5. cancel
    let seq = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    client.publish(&request_topic, QoS::AtMostOnce, false, format!(r#"{{"print":{{"command":"stop","param":"","sequence_id":"{seq}"}}}}"#)).await.ok();
    println!("<<< stop sent");
    std::fs::remove_file(&sliced_path).ok();
}
