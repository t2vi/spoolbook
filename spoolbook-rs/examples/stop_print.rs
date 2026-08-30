// Emergency / manual cancel: connect, publish the `stop` command, watch gcode_state settle.
//
//   cargo run --example stop_print -- <ip> <access_code> <serial>
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use std::time::Duration;

#[derive(Debug)]
struct NoCertVerification;

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
        rustls::crypto::CryptoProvider::get_default().map(|p| p.signature_verification_algorithms.supported_schemes()).unwrap_or_default()
    }
}

fn tls_config() -> rustls::ClientConfig {
    rustls::ClientConfig::builder().dangerous().with_custom_certificate_verifier(std::sync::Arc::new(NoCertVerification)).with_no_client_auth()
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, ip, access_code, serial] = args.as_slice() else {
        eprintln!("usage: stop_print <ip> <access_code> <serial>");
        std::process::exit(1);
    };

    let mut mqttoptions = MqttOptions::new(format!("spoolbook-stop-{}", std::process::id()), ip.clone(), 8883);
    mqttoptions.set_credentials("bblp", access_code);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(std::sync::Arc::new(tls_config()))));
    mqttoptions.set_max_packet_size(1024 * 1024, 1024 * 1024);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    let report_topic = format!("device/{serial}/report");
    let request_topic = format!("device/{serial}/request");
    client.subscribe(&report_topic, QoS::AtMostOnce).await.expect("subscribe");

    let mut sent = false;
    for _ in 0..40 {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                let seq = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                let payload = format!(r#"{{"print":{{"command":"stop","param":"","sequence_id":"{seq}"}}}}"#);
                client.publish(&request_topic, QoS::AtMostOnce, false, payload).await.expect("publish stop");
                println!("STOP sent (seq {seq})");
                sent = true;
            }
            Ok(Event::Incoming(Packet::Publish(p))) => {
                let body = String::from_utf8_lossy(&p.payload);
                if let Some(i) = body.find("\"gcode_state\"") {
                    println!("gcode_state {}", &body[i..(i + 30).min(body.len())]);
                }
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("poll error: {e}");
                if sent {
                    return;
                }
            }
        }
    }
    println!("done polling");
}
