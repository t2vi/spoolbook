use spoolbook_rs::bambu_mqtt_payload_parser::{is_active_state, parse};

// Trimmed to the fields the parser actually reads, but real key names/types/values captured
// from a live Bambu P2S "report" MQTT message during an active print.
const RUNNING_STATUS_JSON: &str = r##"{
    "print": {
        "gcode_state": "RUNNING",
        "task_id": "1725",
        "nozzle_temper": 240.0,
        "nozzle_target_temper": 240.0,
        "bed_temper": 70.0,
        "bed_target_temper": 70.0,
        "mc_percent": 8,
        "layer_num": 21,
        "total_layer_num": 908,
        "ams": {
            "tray_now": "0"
        }
    }
}"##;

#[test]
fn parse_extracts_core_fields_from_running_status() {
    let result = parse(RUNNING_STATUS_JSON).unwrap();

    assert_eq!(result.gcode_state, "RUNNING");
    assert_eq!(result.task_id.as_deref(), Some("1725"));
    assert_eq!(result.reading.nozzle_temp_c, Some(240.0));
    assert_eq!(result.reading.bed_temp_c, Some(70.0));
    assert_eq!(result.reading.ams_slot.as_deref(), Some("0"));
    assert_eq!(result.reading.progress_pct, Some(8));
}

// Community-documented Bambu AMS schema (not yet verified against a captured payload from this
// user's own P2S) — confirm field names against a live capture before shipping AMS UI on top.
const RUNNING_STATUS_WITH_FULL_AMS_JSON: &str = r##"{
    "print": {
        "gcode_state": "RUNNING",
        "task_id": "1725",
        "nozzle_temper": 240.0,
        "bed_temper": 70.0,
        "mc_percent": 8,
        "ams": {
            "tray_now": "0",
            "ams": [
                {
                    "id": "0",
                    "humidity": "5",
                    "tray": [
                        { "id": "0", "tray_type": "PLA", "tray_color": "FFFFFFFF", "remain": 72 },
                        { "id": "1", "tray_type": "PETG", "tray_color": "1A1A1AFF", "remain": 40 },
                        { "id": "2", "tray_type": "", "tray_color": "", "remain": -1 },
                        { "id": "3", "tray_type": "ABS", "tray_color": "FF0000FF", "remain": 15 }
                    ]
                }
            ]
        }
    }
}"##;

#[test]
fn parse_extracts_full_ams_inventory_when_present() {
    let result = parse(RUNNING_STATUS_WITH_FULL_AMS_JSON).unwrap();

    assert_eq!(result.ams_units.len(), 1);
    let unit = &result.ams_units[0];
    assert_eq!(unit.unit_id, "0");
    assert_eq!(unit.humidity_level, Some(5));
    assert_eq!(unit.trays.len(), 4);

    assert_eq!(unit.trays[0].material_type.as_deref(), Some("PLA"));
    assert_eq!(unit.trays[0].color_hex.as_deref(), Some("FFFFFFFF"));
    assert_eq!(unit.trays[0].remain_percent, Some(72));

    assert_eq!(unit.trays[2].material_type, None);
    assert_eq!(unit.trays[2].color_hex, None);
    assert_eq!(unit.trays[2].remain_percent, None, "-1 is Bambu's unknown sentinel");
}

#[test]
fn parse_returns_empty_ams_units_when_ams_array_absent() {
    let result = parse(RUNNING_STATUS_JSON).unwrap();
    assert_eq!(result.ams_units.len(), 0);
}

#[test]
fn parse_leaves_chamber_temp_none_when_field_absent() {
    let result = parse(RUNNING_STATUS_JSON).unwrap();
    assert_eq!(result.reading.chamber_temp_c, None);
}

#[test]
fn parse_returns_none_when_no_print_key() {
    assert!(parse(r#"{ "system": { "sequence_id": "1" } }"#).is_none());
}

#[test]
fn parse_returns_none_when_gcode_state_missing() {
    assert!(parse(r#"{ "print": { "nozzle_temper": 210.0 } }"#).is_none());
}

#[test]
fn parse_returns_none_for_malformed_json() {
    assert!(parse("not json").is_none());
}

#[test]
fn is_active_state_classifies_gcode_state() {
    assert!(is_active_state("RUNNING"));
    assert!(is_active_state("PAUSE"));
    assert!(is_active_state("PREPARE"));
    assert!(is_active_state("running"));
    assert!(!is_active_state("IDLE"));
    assert!(!is_active_state("FINISH"));
    assert!(!is_active_state("FAILED"));
}
