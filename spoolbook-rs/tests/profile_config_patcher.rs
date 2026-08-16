use serde_json::Value;
use spoolbook_rs::profile_config_patcher::patch;
use spoolbook_rs::profiles::PrintProfile;

// Same fixture as Spoolbook.Desktop.Tests/Fixtures/project_settings_sample.json — a trimmed
// real project_settings.config export, not synthetic data.
const FIXTURE: &str = r##"{
  "nozzle_temperature": ["240", "240"],
  "nozzle_temperature_initial_layer": ["255", "245"],
  "hot_plate_temp": ["70"],
  "fan_min_speed": ["20"],
  "fan_max_speed": ["40"],
  "retraction_length": ["0.8", "0.8"],
  "wipe": ["1", "1"],
  "cooling_slowdown_logic": ["uniform_cooling"],
  "layer_height": "0.2",
  "sparse_infill_density": "15%",
  "printer_model": "Bambu Lab P2S",
  "filament_type": ["PETG"],
  "required_nozzle_HRC": ["3"],
  "z_hop_types": ["Auto Lift", "Auto Lift"],
  "long_retractions_when_cut": ["0", "0"]
}"##;

fn minimal_profile() -> PrintProfile {
    PrintProfile { name: "Test".to_string(), filament_id: 1, nozzle_temp_c: 240, ..Default::default() }
}

#[test]
fn sets_scalar_field_mapped_to_array_key_at_index_zero_only() {
    let mut profile = minimal_profile();
    profile.nozzle_temp_c = 250;

    let result: Value = serde_json::from_str(&patch(FIXTURE, &profile)).unwrap();

    assert_eq!(result["nozzle_temperature"][0], "250");
    assert_eq!(result["nozzle_temperature"][1], "240", "second filament's value untouched");
}

#[test]
fn preserves_two_element_arrays_at_index_zero() {
    let mut profile = minimal_profile();
    profile.nozzle_temp_initial_c = Some(260);

    let result: Value = serde_json::from_str(&patch(FIXTURE, &profile)).unwrap();

    assert_eq!(result["nozzle_temperature_initial_layer"][0], "260");
    assert_eq!(result["nozzle_temperature_initial_layer"][1], "245");
}

#[test]
fn converts_bool_to_zero_one_not_true_false() {
    let mut profile = minimal_profile();
    profile.long_retractions_when_cut = Some(true);

    let result: Value = serde_json::from_str(&patch(FIXTURE, &profile)).unwrap();

    assert_eq!(result["long_retractions_when_cut"][0], "1");
}

#[test]
fn leaves_null_fields_original_value_untouched() {
    let profile = minimal_profile(); // hot_plate_temp_c left None

    let result: Value = serde_json::from_str(&patch(FIXTURE, &profile)).unwrap();

    assert_eq!(result["hot_plate_temp"][0], "70", "fixture's original value, unchanged");
}

#[test]
fn leaves_keys_not_owned_by_print_profile_untouched() {
    let mut profile = minimal_profile();
    profile.nozzle_temp_c = 999;

    let result: Value = serde_json::from_str(&patch(FIXTURE, &profile)).unwrap();

    assert_eq!(result["layer_height"], "0.2");
    assert_eq!(result["sparse_infill_density"], "15%");
    assert_eq!(result["printer_model"], "Bambu Lab P2S");
}

#[test]
fn sets_a_plain_optional_int_field_at_index_zero() {
    let mut profile = minimal_profile();
    profile.required_nozzle_hrc = Some(5);

    let result: Value = serde_json::from_str(&patch(FIXTURE, &profile)).unwrap();

    assert_eq!(result["required_nozzle_HRC"][0], "5");
}
