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
    assert_eq!(result.reading.layer_num, Some(21));
    assert_eq!(result.reading.total_layer_num, Some(908));
}

// Field names/types confirmed against maziggy/bambuddy (a mature, actively-maintained Bambu
// self-host project) rather than this user's own P2S directly -- humidity_raw + humidity as
// sibling string fields matches their backend/tests/unit/services/test_bambu_mqtt.py fixtures
// exactly (line 6306: humidity_raw "16"; line 692: humidity "3").
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
                    "humidity_raw": "38",
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
    assert_eq!(result.reading.ams_humidity_pct, Some(38), "reading snapshot mirrors the first AMS unit's real-% humidity");
    let unit = &result.ams_units[0];
    assert_eq!(unit.unit_id, "0");
    assert_eq!(unit.humidity_pct, Some(38), "humidity_raw (real %) preferred over the coarse index");
    assert_eq!(unit.humidity_level, Some(5));
    assert_eq!(unit.trays.len(), 4);

    assert_eq!(unit.trays[0].material_type.as_deref(), Some("PLA"));
    assert_eq!(unit.trays[0].color_hex.as_deref(), Some("FFFFFFFF"));
    assert_eq!(unit.trays[0].remain_percent, Some(72));

    assert_eq!(unit.trays[2].material_type, None);
    assert_eq!(unit.trays[2].color_hex, None);
    assert_eq!(unit.trays[2].remain_percent, None, "-1 is Bambu's unknown sentinel");
}

// Older/original AMS units have no hygrometer, so the printer never sends humidity_raw for
// them -- only the coarse 1-5 LED-ring index.
#[test]
fn parse_falls_back_to_the_coarse_index_when_humidity_raw_is_absent() {
    let json = r##"{
        "print": {
            "gcode_state": "RUNNING",
            "ams": {
                "tray_now": "0",
                "ams": [{ "id": "0", "humidity": "3", "tray": [] }]
            }
        }
    }"##;

    let result = parse(json).unwrap();
    let unit = &result.ams_units[0];
    assert_eq!(unit.humidity_pct, None);
    assert_eq!(unit.humidity_level, Some(3));
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
fn parse_has_no_errors_when_print_error_zero_and_no_hms() {
    assert!(parse(RUNNING_STATUS_JSON).unwrap().errors.is_empty());
}

#[test]
fn parse_decodes_the_blocking_print_error_and_hms_warnings() {
    // print_error 134184978 == 0x07FF8012 -> "07FF-8012" (the AMS mapping failure the P2S
    // pauses on). hms entry attr 0x05000000 / code 0x00008092 -> "0500-8092" (camera, non-fatal).
    let json = r##"{
        "print": {
            "gcode_state": "PAUSE",
            "print_error": 134184978,
            "hms": [
                { "attr": 83886080, "code": 32914 },
                { "attr": 0, "code": 0 }
            ]
        }
    }"##;

    let errors = parse(json).unwrap().errors;
    assert_eq!(errors.len(), 2);

    assert_eq!(errors[0].code, "07FF-8012");
    assert!(errors[0].blocking);
    assert!(errors[0].message.contains("AMS mapping table"));
    assert_eq!(errors[0].wiki_url, "https://wiki.bambulab.com/en/hms/07FF-8012");

    assert_eq!(errors[1].code, "0500-8092");
    assert!(!errors[1].blocking);
    assert!(errors[1].message.contains("camera"));
}

#[test]
fn parse_gives_unknown_codes_a_generic_message_not_a_bare_number() {
    let json = r##"{ "print": { "gcode_state": "PAUSE", "print_error": 305419896 } }"##;
    let err = &parse(json).unwrap().errors[0];
    assert_eq!(err.code, "1234-5678");
    assert!(err.message.contains("1234-5678"));
    assert!(err.message.len() > "1234-5678".len(), "must add context, not just echo the code");
    assert_eq!(err.wiki_url, "https://wiki.bambulab.com/en/hms/1234-5678");
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
