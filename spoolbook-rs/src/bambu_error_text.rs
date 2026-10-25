// Printer error text from Bambu's own error dictionary — the per-model list Bambu Studio
// downloads (HMSQueryMgr) — instead of a hand-kept table. Fetched once per printer model in the
// background; until it lands, errors still show with a generic message and Bambu's link.
use crate::bambu_mqtt_payload_parser::PrinterError;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DescribedError {
    pub code: String,
    pub message: String,
    pub blocking: bool,
    /// Listed by Bambu with no text — Bambu Studio hides these; we show them, flagged.
    pub internal: bool,
    pub wiki_url: String,
}

fn query_url() -> String {
    std::env::var("BAMBU_HMS_URL").unwrap_or_else(|_| "https://e.bambulab.com/query.php".to_string())
}

// ponytail: one map for both lists — device_error ecodes are 8 hex, device_hms 16, so keys never collide.
pub fn parse_dictionary(json: &str) -> Result<HashMap<String, String>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = HashMap::new();
    for block in ["device_error", "device_hms"] {
        for item in v["data"][block]["en"].as_array().into_iter().flatten() {
            if let (Some(ecode), Some(intro)) = (item["ecode"].as_str(), item["intro"].as_str()) {
                out.insert(ecode.to_uppercase(), intro.to_string());
            }
        }
    }
    if out.is_empty() { Err("no error codes in Bambu's dictionary response".to_string()) } else { Ok(out) }
}

pub fn lookup(texts: Option<&HashMap<String, String>>, serial: Option<&str>, err: &PrinterError) -> DescribedError {
    let (message, internal) = match texts.and_then(|t| t.get(&err.ecode)) {
        Some(text) if !text.is_empty() => (text.clone(), false),
        Some(_) => (format!("Internal Bambu code {} — Bambu publishes no description for it.", err.code), true),
        None => (format!("Printer reported {}. See the linked Bambu page for details.", err.code), false),
    };
    // Bambu's redirector picks the model's own wiki page from the serial (Bambu Studio's
    // get_hms_wiki_url), falling back to the wiki's HMS home for codes without one.
    let kind = if err.blocking { "device_error" } else { "device_hms" };
    let device = serial.map(|s| format!("&d={s}")).unwrap_or_default();
    DescribedError {
        code: err.code.clone(),
        message,
        blocking: err.blocking,
        internal,
        wiki_url: format!("https://e.bambulab.com/index.php?e={}{device}&s={kind}&lang=en", err.ecode),
    }
}

const REFRESH_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY_AFTER: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
struct Entry {
    texts: Option<Arc<HashMap<String, String>>>,
    attempted: Option<Instant>,
}

// Keyed by model (first 3 chars of the serial, as Bambu Studio does). Process-global: the
// dictionary is public, per model, and the same for every printer of that model.
static CACHE: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(Default::default);

/// Never waits on the network: returns what's cached now and kicks off a fetch when due.
/// Must be called inside a tokio runtime.
pub fn describe(serial: Option<&str>, errors: &[PrinterError]) -> Vec<DescribedError> {
    if errors.is_empty() {
        return Vec::new();
    }
    let texts = serial.and_then(|s| s.get(..3)).and_then(texts_for);
    errors.iter().map(|e| lookup(texts.as_deref(), serial, e)).collect()
}

fn texts_for(model: &str) -> Option<Arc<HashMap<String, String>>> {
    let mut cache = CACHE.lock().unwrap();
    let entry = cache.entry(model.to_string()).or_default();
    let due = match (entry.attempted, &entry.texts) {
        (None, _) => true,
        (Some(at), Some(_)) => at.elapsed() > REFRESH_AFTER,
        (Some(at), None) => at.elapsed() > RETRY_AFTER,
    };
    if due {
        entry.attempted = Some(Instant::now());
        tokio::spawn(refresh(model.to_string()));
    }
    entry.texts.clone()
}

async fn refresh(model: String) {
    match fetch(&model).await {
        Ok(texts) => CACHE.lock().unwrap().entry(model).or_default().texts = Some(Arc::new(texts)),
        // Keeps whatever was cached; texts_for retries after RETRY_AFTER.
        Err(e) => eprintln!("[hms] error dictionary fetch for model {model} failed: {e}"),
    }
}

async fn fetch(model: &str) -> Result<HashMap<String, String>, String> {
    let mut url = reqwest::Url::parse(&query_url()).map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("lang", "en").append_pair("d", model);
    let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build().map_err(|e| e.to_string())?;
    let body = client.get(url).send().await.and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    parse_dictionary(&body)
}
