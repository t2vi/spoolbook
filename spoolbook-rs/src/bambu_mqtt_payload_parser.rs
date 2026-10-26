use serde::Serialize;
use serde_json::Value;

pub struct ReadingInput {
    pub nozzle_temp_c: Option<f64>,
    pub bed_temp_c: Option<f64>,
    pub chamber_temp_c: Option<f64>,
    pub ams_slot: Option<String>,
    pub progress_pct: Option<i64>,
    // Mirrors the first AMS unit's real-% humidity (see snapshot_for_end_job in printer_mqtt.rs
    // for the same first-unit convention) -- coarse humidity_level is never stored here, same
    // real-percentage-only rule as the end-of-print snapshot.
    pub ams_humidity_pct: Option<i64>,
    pub layer_num: Option<i64>,
    pub total_layer_num: Option<i64>,
}

// Live-only AMS snapshot — deliberately not persisted onto PrinterReading: the
// point is a live status view, not a per-tray time series in print history. Clone+Serialize so
// printer_mqtt.rs's live-status store can hand a snapshot straight to the /live SSE endpoint.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmsTrayReading {
    pub slot_id: String,
    pub material_type: Option<String>,
    pub color_hex: Option<String>,
    pub remain_percent: Option<i64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmsUnitReading {
    pub unit_id: String,
    // Bambu's newer AMS units (AMS 2 Pro, AMS-HT -- what a P2S ships with) have a real hygrometer
    // and report it as humidity_raw, a genuine relative-humidity percentage. Older/original AMS
    // units only have humidity: a coarse 1-5 index driving the physical unit's LED ring, not a
    // percentage. Confirmed against maziggy/bambuddy (backend/app/services/printer_manager.py),
    // which resolves the same way: prefer humidity_raw, humidity is index-only fallback.
    pub humidity_pct: Option<i64>,
    pub humidity_level: Option<i64>,
    pub trays: Vec<AmsTrayReading>,
}

pub struct BambuTelemetryMessage {
    pub gcode_state: String,
    pub task_id: Option<String>,
    pub reading: ReadingInput,
    pub ams_units: Vec<AmsUnitReading>,
    // Everything the printer is complaining about right now: the one blocking `print_error`
    // (what pops the modal in Bambu Studio) first, then any `hms` warnings. Empty once the
    // printer reports `print_error: 0` and an empty `hms` array again.
    pub errors: Vec<PrinterError>,
}

// One thing the printer is complaining about, in Bambu Studio's own code format. Text and link
// come from Bambu's error dictionary (bambu_error_text.rs), keyed on `ecode`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PrinterError {
    /// As shown: `07FF-8012` for a print_error, `0500-0300-0002-0070` for an hms entry.
    pub code: String,
    /// The dictionary key: the same hex without dashes (8 digits print_error, 16 hms).
    pub ecode: String,
    /// `true` for the `print_error` that halts the print; `false` for an `hms` warning.
    pub blocking: bool,
}

fn make_error(ecode: String, blocking: bool) -> PrinterError {
    let code = ecode.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join("-");
    PrinterError { code, ecode, blocking }
}

fn parse_errors(print: &Value) -> Vec<PrinterError> {
    let mut out = Vec::new();

    // The blocking one first. `print_error` is an int; 0 (or absent) means no error.
    let raw = print
        .get("print_error")
        .and_then(Value::as_u64)
        .or_else(|| print.get("print_error").and_then(Value::as_str).and_then(|s| s.parse().ok()))
        .unwrap_or(0);
    if raw != 0 {
        out.push(make_error(format!("{raw:08X}"), true));
    }

    // Then hms warnings (attr+code pairs). De-dupe against the blocking code and each other.
    if let Some(arr) = print.get("hms").and_then(Value::as_array) {
        for entry in arr {
            let (Some(attr), Some(code)) = (entry.get("attr").and_then(Value::as_u64), entry.get("code").and_then(Value::as_u64)) else {
                continue;
            };
            if attr == 0 && code == 0 {
                continue;
            }
            // Bambu Studio's DevHMSItem::get_long_error_code: attr, then code, all 32 bits each.
            let ecode = format!("{attr:08X}{code:08X}");
            if out.iter().any(|e| e.ecode == ecode) {
                continue;
            }
            out.push(make_error(ecode, false));
        }
    }

    out
}

const ACTIVE_STATES: &[&str] = &["RUNNING", "PAUSE", "PREPARE"];

pub fn is_active_state(gcode_state: &str) -> bool {
    ACTIVE_STATES.iter().any(|s| s.eq_ignore_ascii_case(gcode_state))
}

// Parses Bambu Lab's LAN MQTT "report" topic payload (device/{serial}/report) — pure JSON-in,
// struct-out, no network/DB, so it's exercised directly by tests against captured real payloads.
// Bambu's broker also emits partial/delta messages that omit gcode_state entirely — treated as
// not a usable status snapshot (returns None) rather than guessing at missing state.
pub fn parse(json: &str) -> Option<BambuTelemetryMessage> {
    let doc: Value = serde_json::from_str(json).ok()?;
    let print = doc.get("print")?;
    let gcode_state = print.get("gcode_state")?.as_str()?.to_string();

    let mut ams_slot = None;
    let mut ams_units = Vec::new();
    if let Some(ams) = print.get("ams") {
        ams_slot = ams.get("tray_now").and_then(Value::as_str).map(String::from);

        if let Some(arr) = ams.get("ams").and_then(Value::as_array) {
            ams_units.extend(arr.iter().map(parse_ams_unit));
        }
    }

    let ams_humidity_pct = ams_units.first().and_then(|u| u.humidity_pct);

    Some(BambuTelemetryMessage {
        gcode_state,
        task_id: print.get("task_id").and_then(Value::as_str).map(String::from),
        errors: parse_errors(print),
        reading: ReadingInput {
            nozzle_temp_c: print.get("nozzle_temper").and_then(Value::as_f64),
            bed_temp_c: print.get("bed_temper").and_then(Value::as_f64),
            chamber_temp_c: print.get("chamber_temper").and_then(Value::as_f64),
            ams_slot,
            progress_pct: print.get("mc_percent").and_then(Value::as_i64),
            ams_humidity_pct,
            layer_num: print.get("layer_num").and_then(Value::as_i64),
            total_layer_num: print.get("total_layer_num").and_then(Value::as_i64),
        },
        ams_units,
    })
}

fn parse_ams_unit(unit_el: &Value) -> AmsUnitReading {
    let mut trays = Vec::new();
    if let Some(arr) = unit_el.get("tray").and_then(Value::as_array) {
        for tray_el in arr {
            let material_type = tray_el.get("tray_type").and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from);
            let color_hex = tray_el.get("tray_color").and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from);
            // -1 is Bambu's "unknown" sentinel (untagged spool / no RFID read yet)
            let remain_percent = tray_el.get("remain").and_then(Value::as_i64).filter(|&r| r != -1);

            trays.push(AmsTrayReading {
                slot_id: tray_el.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
                material_type,
                color_hex,
                remain_percent,
            });
        }
    }

    AmsUnitReading {
        unit_id: unit_el.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
        // Both reported as string digits, not numbers, unlike most other fields here.
        humidity_pct: unit_el.get("humidity_raw").and_then(Value::as_str).and_then(|s| s.parse().ok()),
        humidity_level: unit_el.get("humidity").and_then(Value::as_str).and_then(|s| s.parse().ok()),
        trays,
    }
}
