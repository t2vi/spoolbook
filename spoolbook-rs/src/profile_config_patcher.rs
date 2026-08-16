use crate::profile_field_spec::fmt_f64;
use crate::profiles::PrintProfile;
use serde_json::Value;

// Port of ProfileConfigPatcher.cs (Spoolbook.Desktop) — patches a real Project's
// Metadata/project_settings.config (verbatim Bambu Studio config JSON) with a PrintProfile's
// fields, for the re-slice-before-print feature. Patches, never regenerates: every key not
// listed below (machine/printer settings, start/end gcode not owned by PrintProfile, bed shape,
// etc.) survives untouched. Every mapped key is array-valued in the real config (per-filament)
// and is patched at index 0 only — spoolbook only ever targets a single AMS slot per print. The
// C# source keeps an `IsArray` bool per entry for a scalar-key branch that's never exercised (every
// entry is `true`); dropped here since nothing in the real config or its tests needs it.
pub fn patch(original_json: &str, p: &PrintProfile) -> String {
    let mut root: Value = serde_json::from_str(original_json).expect("invalid config json");
    let obj = root.as_object_mut().expect("config json root must be an object");

    macro_rules! set {
        ($key:literal, $value:expr) => {
            if let Some(v) = $value {
                patch_array(obj, $key, v);
            }
        };
    }
    macro_rules! b {
        ($key:literal, $field:ident) => {
            set!($key, p.$field.map(|v| if v { "1".to_string() } else { "0".to_string() }));
        };
    }
    macro_rules! i {
        ($key:literal, $field:ident) => {
            set!($key, p.$field.map(|v| v.to_string()));
        };
    }
    macro_rules! fl {
        ($key:literal, $field:ident) => {
            set!($key, p.$field.map(fmt_f64));
        };
    }
    macro_rules! s {
        ($key:literal, $field:ident) => {
            set!($key, p.$field.clone());
        };
    }

    patch_array(obj, "nozzle_temperature", p.nozzle_temp_c.to_string());

    b!("activate_air_filtration", activate_air_filtration);
    b!("filament_adaptive_volumetric_speed", adaptive_volumetric_speed);
    i!("additional_cooling_fan_speed", additional_cooling_fan_speed_pct);
    i!("filament_adhesiveness_category", adhesiveness_category);
    fl!("bridge_speed", bridge_speed_mm_s);
    i!("chamber_temperatures", chamber_temperature_c);
    fl!("filament_change_length", change_length_mm);
    fl!("filament_change_length_nc", change_length_nc_mm);
    fl!("circle_compensation_speed", circle_compensation_speed_mm_s);
    i!("close_fan_the_first_x_layers", close_fan_first_x_layers);
    i!("complete_print_exhaust_fan_speed", complete_print_exhaust_fan_speed_pct);
    i!("cool_plate_temp", cool_plate_temp_c);
    i!("cool_plate_temp_initial_layer", cool_plate_temp_initial_c);
    i!("filament_cooling_before_tower", cooling_before_tower_s);
    fl!("cooling_perimeter_transition_distance", cooling_perimeter_transition_distance_mm);
    s!("cooling_slowdown_logic", cooling_slowdown_logic);
    fl!("filament_cost", cost_per_kg);
    fl!("counter_coef_1", counter_coef_1);
    fl!("counter_coef_2", counter_coef_2);
    fl!("counter_coef_3", counter_coef_3);
    fl!("counter_limit_max", counter_limit_max);
    fl!("counter_limit_min", counter_limit_min);
    s!("default_filament_colour", default_colour_hex);
    fl!("filament_density", density_g_cm3);
    fl!("deretraction_speed", deretraction_speed_mm_s);
    fl!("diameter_limit", diameter_limit_mm);
    fl!("filament_diameter", diameter_mm);
    i!("filament_dev_ams_drying_heat_distortion_temperature", drying_ams_heat_distortion_temp_c);
    s!("filament_dev_ams_drying_ams_limitations", drying_ams_limitations);
    i!("filament_dev_ams_drying_temperature", drying_ams_temp_c);
    fl!("filament_dev_ams_drying_time", drying_ams_time_h);
    i!("filament_dev_chamber_drying_bed_temperature", drying_chamber_bed_temp_c);
    fl!("filament_dev_chamber_drying_time", drying_chamber_time_h);
    i!("filament_dev_drying_cooling_temperature", drying_cooling_temp_c);
    i!("filament_dev_drying_softening_temperature", drying_softening_temp_c);
    i!("during_print_exhaust_fan_speed", during_print_exhaust_fan_speed_pct);
    b!("enable_overhang_bridge_fan", enable_overhang_bridge_fan);
    b!("enable_overhang_speed", enable_overhang_speed);
    b!("enable_pressure_advance", enable_pressure_advance);
    s!("filament_end_gcode", end_gcode);
    i!("eng_plate_temp", eng_plate_temp_c);
    i!("eng_plate_temp_initial_layer", eng_plate_temp_initial_c);
    s!("filament_extruder_variant", extruder_variant);
    i!("fan_cooling_layer_time", fan_cooling_layer_time_s);
    i!("fan_max_speed", fan_max_speed_pct);
    i!("fan_min_speed", fan_min_speed_pct);
    i!("first_x_layer_fan_speed", first_x_layer_fan_speed_pct);
    fl!("filament_flow_ratio", flow_ratio);
    i!("filament_flush_temp", flush_temp_c);
    fl!("filament_flush_volumetric_speed", flush_volumetric_speed_mm3_s);
    i!("full_fan_speed_layer", full_fan_speed_layer);
    fl!("hole_coef_1", hole_coef_1);
    fl!("hole_coef_2", hole_coef_2);
    fl!("hole_coef_3", hole_coef_3);
    fl!("hole_limit_max", hole_limit_max);
    fl!("hole_limit_min", hole_limit_min);
    i!("hot_plate_temp", hot_plate_temp_c);
    i!("hot_plate_temp_initial_layer", hot_plate_temp_initial_c);
    fl!("impact_strength_z", impact_strength_z);
    b!("filament_is_support", is_support);
    b!("long_retractions_when_cut", long_retractions_when_cut);
    fl!("filament_max_volumetric_speed", max_volumetric_speed_mm3_s);
    fl!("filament_minimal_purge_on_wipe_tower", minimal_purge_on_wipe_tower_mm3);
    b!("no_slow_down_for_cooling_on_outwalls", no_slow_down_for_cooling_on_outwalls);
    i!("nozzle_temperature_initial_layer", nozzle_temp_initial_c);
    i!("nozzle_temperature_range_high", nozzle_temp_range_high_c);
    i!("nozzle_temperature_range_low", nozzle_temp_range_low_c);
    fl!("overhang_1_4_speed", overhang_14_speed_mm_s);
    fl!("overhang_2_4_speed", overhang_24_speed_mm_s);
    fl!("overhang_3_4_speed", overhang_34_speed_mm_s);
    fl!("overhang_4_4_speed", overhang_44_speed_mm_s);
    i!("overhang_fan_speed", overhang_fan_speed_pct);
    s!("overhang_fan_threshold", overhang_fan_threshold);
    s!("overhang_threshold_participating_cooling", overhang_threshold_participating_cooling);
    fl!("overhang_totally_speed", overhang_totally_speed_mm_s);
    b!("override_process_overhang_speed", override_process_overhang_speed);
    i!("pre_start_fan_time", pre_start_fan_time_s);
    fl!("pressure_advance", pressure_advance);
    fl!("filament_prime_volume", prime_volume_mm3);
    fl!("filament_prime_volume_nc", prime_volume_nc_mm3);
    i!("filament_printable", printable);
    fl!("filament_ramming_travel_time_nc", ramming_travel_time_nc_s);
    fl!("filament_ramming_travel_time", ramming_travel_time_s);
    fl!("filament_ramming_volumetric_speed", ramming_volumetric_speed_mm3_s);
    fl!("filament_ramming_volumetric_speed_nc", ramming_volumetric_speed_nc_mm3_s);
    b!("reduce_fan_stop_start_freq", reduce_fan_stop_start_freq);
    i!("required_nozzle_HRC", required_nozzle_hrc);
    s!("retract_before_wipe", retract_before_wipe);
    fl!("retract_length_toolchange", retract_length_nc_mm);
    fl!("retract_restart_extra", retract_restart_extra_mm);
    b!("retract_when_changing_layer", retract_when_changing_layer);
    fl!("retraction_distances_when_cut", retraction_distances_when_cut_mm);
    fl!("retraction_distances_when_ec", retraction_distances_when_ec_mm);
    fl!("retraction_minimum_travel", retraction_minimum_travel_mm);
    fl!("retraction_length", retraction_mm);
    fl!("retraction_speed", retraction_speed_mm_s);
    s!("filament_scarf_gap", scarf_gap_pct);
    s!("filament_scarf_height", scarf_height_pct);
    fl!("filament_scarf_length", scarf_length_mm);
    s!("filament_scarf_seam_type", scarf_seam_type);
    s!("filament_shrink", shrink_pct);
    b!("slow_down_for_layer_cooling", slow_down_for_layer_cooling);
    i!("slow_down_layer_time", slow_down_layer_time_s);
    i!("slow_down_min_speed", slow_down_min_speed_mm_s);
    b!("filament_soluble", soluble);
    s!("filament_start_gcode", start_gcode);
    i!("supertack_plate_temp", supertack_plate_temp_c);
    i!("supertack_plate_temp_initial_layer", supertack_plate_temp_initial_c);
    i!("textured_plate_temp", textured_plate_temp_c);
    i!("textured_plate_temp_initial_layer", textured_plate_temp_initial_c);
    fl!("filament_tower_interface_pre_extrusion_dist", tower_interface_pre_extrusion_dist_mm);
    fl!("filament_tower_interface_pre_extrusion_length", tower_interface_pre_extrusion_length_mm);
    i!("filament_tower_interface_print_temp", tower_interface_print_temp_c);
    fl!("filament_tower_interface_purge_volume", tower_interface_purge_volume_mm3);
    fl!("filament_tower_ironing_area", tower_ironing_area_mm2);
    fl!("filament_velocity_adaptation_factor", velocity_adaptation_factor);
    s!("volumetric_speed_coefficients", volumetric_speed_coefficients);
    fl!("wipe_distance", wipe_distance_mm);
    b!("wipe", wipe_enabled);
    fl!("z_hop", z_hop_mm);
    s!("z_hop_types", z_hop_type);

    root.to_string()
}

fn patch_array(obj: &mut serde_json::Map<String, Value>, key: &str, value: String) {
    match obj.get_mut(key) {
        Some(Value::Array(arr)) if !arr.is_empty() => arr[0] = Value::String(value),
        _ => {
            obj.insert(key.to_string(), Value::Array(vec![Value::String(value)]));
        }
    }
}
