use serde_json::json;
use spoolbook_rs::bambu_error_text::{describe, lookup, parse_dictionary};
use spoolbook_rs::bambu_mqtt_payload_parser::PrinterError;
use std::collections::HashMap;
use tokio::net::TcpListener;

fn print_error(code: &str, ecode: &str) -> PrinterError {
    PrinterError { code: code.into(), ecode: ecode.into(), blocking: true }
}

fn hms(code: &str, ecode: &str) -> PrinterError {
    PrinterError { code: code.into(), ecode: ecode.into(), blocking: false }
}

fn dictionary() -> serde_json::Value {
    json!({
        "result": 0,
        "data": {
            "device_hms": { "ver": 1, "en": [
                { "ecode": "0500030000010001", "intro": "The MC module is malfunctioning." },
                { "ecode": "0500030000020070", "intro": "" }
            ] },
            "device_error": { "ver": 1, "en": [
                { "ecode": "07ff8012", "intro": "Failed to get AMS mapping table." }
            ] }
        }
    })
}

fn texts() -> HashMap<String, String> {
    parse_dictionary(&dictionary().to_string()).unwrap()
}

// spec: 004/FR-014
#[test]
fn listed_code_shows_bambus_text_case_insensitively() {
    let texts = texts();
    let err = lookup(Some(&texts), Some("22E8AJ5C0700123"), &print_error("07FF-8012", "07FF8012"));
    assert_eq!(err.message, "Failed to get AMS mapping table.");
    assert!(!err.internal);
    assert!(err.blocking);
    assert_eq!(err.code, "07FF-8012");
}

// spec: 004/FR-014
#[test]
fn blank_text_is_shown_flagged_internal() {
    let texts = texts();
    let err = lookup(Some(&texts), Some("22E8AJ5C0700123"), &hms("0500-0300-0002-0070", "0500030000020070"));
    assert!(err.internal);
    assert!(err.message.contains("no description"), "{}", err.message);
}

// spec: 004/FR-014
#[test]
fn unlisted_or_no_dictionary_gets_a_generic_message_with_the_code() {
    let texts = texts();
    let unlisted = lookup(Some(&texts), Some("22E8AJ5C0700123"), &hms("0C00-0100-0001-0004", "0C00010000010004"));
    let unloaded = lookup(None, None, &print_error("1234-5678", "12345678"));
    for err in [unlisted, unloaded] {
        assert!(!err.internal);
        assert!(err.message.contains(&err.code), "{}", err.message);
        assert!(err.message.len() > err.code.len(), "must add context, not just echo the code");
    }
}

// spec: 004/FR-012
#[test]
fn links_to_bambus_redirector_for_the_exact_code_and_printer() {
    let texts = texts();
    let serial = Some("22E8AJ5C0700123");
    assert_eq!(
        lookup(Some(&texts), serial, &hms("0500-0300-0001-0001", "0500030000010001")).wiki_url,
        "https://e.bambulab.com/index.php?e=0500030000010001&d=22E8AJ5C0700123&s=device_hms&lang=en"
    );
    assert_eq!(
        lookup(Some(&texts), serial, &print_error("07FF-8012", "07FF8012")).wiki_url,
        "https://e.bambulab.com/index.php?e=07FF8012&d=22E8AJ5C0700123&s=device_error&lang=en"
    );
    assert_eq!(
        lookup(None, None, &print_error("07FF-8012", "07FF8012")).wiki_url,
        "https://e.bambulab.com/index.php?e=07FF8012&s=device_error&lang=en"
    );
}

// spec: 004/FR-014
#[tokio::test]
async fn fetches_the_models_dictionary_in_the_background() {
    use axum::extract::{Query, State};
    use std::sync::{Arc, Mutex};

    type Seen = Arc<Mutex<Vec<HashMap<String, String>>>>;
    let seen: Seen = Default::default();
    let body = dictionary();
    let app = axum::Router::new()
        .route(
            "/query.php",
            axum::routing::get(
                |State((seen, body)): State<(Seen, serde_json::Value)>,
                 Query(q): Query<HashMap<String, String>>| async move {
                    seen.lock().unwrap().push(q);
                    axum::Json(body)
                },
            ),
        )
        .with_state((seen.clone(), body));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    unsafe { std::env::set_var("BAMBU_HMS_URL", format!("http://{addr}/query.php")) };

    let errors = [print_error("07FF-8012", "07FF8012")];
    // Not loaded yet: generic text straight away, never waiting on the fetch.
    let first = describe(Some("Q9Z0000000001"), &errors);
    assert!(first[0].message.contains("07FF-8012"));

    let mut text = String::new();
    for _ in 0..50 {
        text = describe(Some("Q9Z0000000001"), &errors)[0].message.clone();
        if text == "Failed to get AMS mapping table." {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(text, "Failed to get AMS mapping table.");

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "one fetch per model, not per call");
    assert_eq!(seen[0].get("d").map(String::as_str), Some("Q9Z"));
    assert_eq!(seen[0].get("lang").map(String::as_str), Some("en"));
}
