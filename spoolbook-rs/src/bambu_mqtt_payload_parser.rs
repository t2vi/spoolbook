use serde_json::Value;

pub struct ReadingInput {
    pub nozzle_temp_c: Option<f64>,
    pub bed_temp_c: Option<f64>,
    pub chamber_temp_c: Option<f64>,
    pub ams_slot: Option<String>,
    pub progress_pct: Option<i64>,
}

// Live-only AMS snapshot — deliberately not persisted onto PrinterReading (docs/adr/0022): the
// point is a live status view, not a per-tray time series in print history.
pub struct AmsTrayReading {
    pub slot_id: String,
    pub material_type: Option<String>,
    pub color_hex: Option<String>,
    pub remain_percent: Option<i64>,
}

pub struct AmsUnitReading {
    pub unit_id: String,
    pub humidity_level: Option<i64>,
    pub trays: Vec<AmsTrayReading>,
}

pub struct BambuTelemetryMessage {
    pub gcode_state: String,
    pub task_id: Option<String>,
    pub reading: ReadingInput,
    pub ams_units: Vec<AmsUnitReading>,
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

    Some(BambuTelemetryMessage {
        gcode_state,
        task_id: print.get("task_id").and_then(Value::as_str).map(String::from),
        reading: ReadingInput {
            nozzle_temp_c: print.get("nozzle_temper").and_then(Value::as_f64),
            bed_temp_c: print.get("bed_temper").and_then(Value::as_f64),
            chamber_temp_c: print.get("chamber_temper").and_then(Value::as_f64),
            ams_slot,
            progress_pct: print.get("mc_percent").and_then(Value::as_i64),
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
        // Bambu reports humidity as a string digit, not a number, unlike most other fields here.
        humidity_level: unit_el.get("humidity").and_then(Value::as_str).and_then(|s| s.parse().ok()),
        trays,
    }
}
