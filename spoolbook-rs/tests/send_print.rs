use serde_json::Value;
use spoolbook_rs::send_print::{build_project_file_payload, compute_gcode_md5, sanitize_for_printer_filename};
use std::io::Write;

#[test]
fn sanitize_strips_extension_and_unsafe_characters() {
    assert_eq!(sanitize_for_printer_filename("My Cool Print (v2)!.3mf"), "MyCoolPrintv2.3mf");
}

#[test]
fn sanitize_preserves_underscores_and_dashes() {
    assert_eq!(sanitize_for_printer_filename("bracket_holder-v3.3mf"), "bracket_holder-v3.3mf");
}

#[test]
fn sanitize_falls_back_to_print_when_nothing_survives() {
    assert_eq!(sanitize_for_printer_filename("★★★.3mf"), "print.3mf");
}

#[test]
fn sanitize_caps_at_sixty_characters() {
    let long_name = format!("{}.3mf", "a".repeat(100));
    let result = sanitize_for_printer_filename(&long_name);
    assert_eq!(result, format!("{}.3mf", "a".repeat(60)));
}

fn write_fixture_3mf_with_gcode(name: &str, gcode_bytes: &[u8]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("Metadata/plate_1.gcode", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(gcode_bytes).unwrap();
    zip.finish().unwrap();
    path
}

#[test]
fn compute_gcode_md5_hashes_only_the_named_plate_gcode_entry() {
    let path = write_fixture_3mf_with_gcode("spoolbook_rs_test_send_print_md5.3mf", b"G28\nG1 X0 Y0\n");
    let expected = format!("{:x}", md5::compute(b"G28\nG1 X0 Y0\n"));

    let result = compute_gcode_md5(path.to_str().unwrap(), "plate_1.gcode");

    assert_eq!(result, Some(expected));
    std::fs::remove_file(path).ok();
}

#[test]
fn compute_gcode_md5_returns_none_when_the_plate_entry_is_missing() {
    let path = write_fixture_3mf_with_gcode("spoolbook_rs_test_send_print_md5_missing.3mf", b"G28\n");

    let result = compute_gcode_md5(path.to_str().unwrap(), "plate_2.gcode");

    assert_eq!(result, None);
    std::fs::remove_file(path).ok();
}

#[test]
fn build_project_file_payload_maps_the_picked_ams_slot_and_forces_vibration_cali_off_for_p2s() {
    // use_ams=true -> ams_mapping is [global_tray_id]. An empty mapping fails HMS 07FF-8012 on a
    // cold P2S (nothing threaded); [slot] with use_ams:true maps gcode filament 0 to that tray.
    let payload = build_project_file_payload("print.3mf", "abc123", "plate_1.gcode", true, 5, true, "42");

    let v: Value = serde_json::from_str(&payload).unwrap();
    let print = &v["print"];

    assert_eq!(print["command"], "project_file");
    assert_eq!(print["param"], "Metadata/plate_1.gcode");
    assert_eq!(print["url"], "ftp:///print.3mf");
    assert_eq!(print["file"], "print.3mf");
    assert_eq!(print["md5"], "abc123");
    assert_eq!(print["use_ams"], true);
    assert_eq!(print["ams_mapping"], serde_json::json!([5]));
    assert_eq!(print["ams_mapping2"], serde_json::json!([]));
    assert_eq!(print["vibration_cali"], false, "P2S doesn't support vibration cali");
    assert_eq!(print["project_id"], "42");
    assert_eq!(print["subtask_id"], "42");
    assert_eq!(print["task_id"], "42");
    assert_eq!(print["subtask_name"], "print");
}

#[test]
fn build_project_file_payload_sends_no_mapping_when_ams_is_off_and_enables_vibration_cali_for_non_p2s() {
    let payload = build_project_file_payload("print.3mf", "abc123", "plate_1.gcode", false, 0, false, "1");

    let v: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(v["print"]["vibration_cali"], true);
    assert_eq!(v["print"]["use_ams"], false);
    assert_eq!(v["print"]["ams_mapping"], serde_json::json!([]));
    assert_eq!(v["print"]["ams_mapping2"], serde_json::json!([]));
}
