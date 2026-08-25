// POC: connect_and_subscribe_loop's Err(_) => break silently discards the real rumqttc error.
// Reproduces just the connect+first-poll step with the error printed, against a real printer.
//
//   cargo run --example mqtt_poc -- <ip> <access_code> <serial>
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, TlsConfiguration, Transport};
use std::time::Duration;

// Copy of printer_mqtt.rs's (pub(crate), not reachable from an example crate) NoCertVerification
// + tls_config() -- self-signed on-device cert, same trust posture as the real thing.
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
        eprintln!("usage: mqtt_poc <ip> <access_code> <serial>");
        std::process::exit(1);
    };

    let mut mqttoptions = MqttOptions::new(format!("spoolbook-poc-{}", std::process::id()), ip.clone(), 8883);
    mqttoptions.set_credentials("bblp", access_code);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(std::sync::Arc::new(tls_config()))));
    mqttoptions.set_max_packet_size(128 * 1024, 128 * 1024);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    let topic = format!("device/{serial}/report");
    println!("subscribing to {topic}...");
    if let Err(e) = client.subscribe(&topic, rumqttc::QoS::AtMostOnce).await {
        eprintln!("subscribe FAILED: {e}");
        std::process::exit(1);
    }

    for i in 0..10 {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(ack))) => println!("[{i}] ConnAck: {ack:?}"),
            Ok(Event::Incoming(Packet::Publish(p))) => {
                println!("[{i}] Publish on {}: {} bytes", p.topic, p.payload.len());
                println!("upload OK -- got a real message, connection works");
                return;
            }
            Ok(ev) => println!("[{i}] event: {ev:?}"),
            Err(e) => {
                eprintln!("[{i}] poll FAILED: {e}");
                std::process::exit(1);
            }
        }
    }
    println!("polled 10 events, no publish yet (printer may just be idle)");
}
