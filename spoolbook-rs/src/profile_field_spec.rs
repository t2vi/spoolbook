use crate::profiles::{ParsedProfileFields, PrintProfile};
use serde::Serialize;
use std::collections::HashMap;

struct FieldDef {
    name: &'static str,
    label: &'static str,
    unit: &'static str,
    is_bool: bool,
    is_text_area: bool,
    is_numeric: bool,
    options: Option<&'static [&'static str]>,
    hide_when_blank: bool,
}

const fn f(name: &'static str, label: &'static str) -> FieldDef {
    FieldDef { name, label, unit: "", is_bool: false, is_text_area: false, is_numeric: false, options: None, hide_when_blank: false }
}
const fn unit(mut d: FieldDef, unit: &'static str) -> FieldDef {
    d.unit = unit;
    d
}
const fn bool_field(mut d: FieldDef) -> FieldDef {
    d.is_bool = true;
    d
}
const fn numeric(mut d: FieldDef) -> FieldDef {
    d.is_numeric = true;
    d
}
const fn text_area(mut d: FieldDef) -> FieldDef {
    d.is_text_area = true;
    d
}
const fn options(mut d: FieldDef, options: &'static [&'static str]) -> FieldDef {
    d.options = Some(options);
    d
}
const fn hide_when_blank(mut d: FieldDef) -> FieldDef {
    d.hide_when_blank = true;
    d
}

// Mirrors Bambu Studio's own tab/section layout (same grouping ProfileFieldSpec.cs and the
// Svelte ProfileForm.svelte's tabs use) — labels here already have their unit suffix split off
// by hand (matching ProfileFieldSpec.cs's SplitUnit regex) rather than parsed at runtime.
const GROUPS: &[(&str, &str, &[FieldDef])] = &[
    ("Filament", "Basic information", &[
        bool_field(f("Soluble", "Soluble material")),
        bool_field(f("IsSupport", "Support material")),
        f("ImpactStrengthZ", "Impact Strength Z"),
        f("RequiredNozzleHrc", "Required nozzle HRC"),
        hide_when_blank(f("DefaultColourHex", "Default color")),
        unit(f("DiameterMm", "Diameter"), "mm"),
        unit(f("DiameterLimitMm", "Diameter limit"), "mm"),
        numeric(f("AdhesivenessCategory", "Adhesiveness Category")),
        f("FlowRatio", "Flow ratio"),
        unit(f("DensityGCm3", "Density"), "g/cm³"),
        unit(f("ShrinkPct", "Shrinkage"), "%"),
        f("VelocityAdaptationFactor", "Velocity Adaptation Factor"),
        unit(f("CostPerKg", "Price"), "money/kg"),
        numeric(unit(f("SofteningTempC", "Softening temperature"), "°C")),
        numeric(f("Printable", "Filament printable")),
        options(f("ExtruderVariant", "Extruder variant"), &["Direct Drive Standard", "Direct Drive High Flow"]),
        unit(f("TowerIroningAreaMm2", "Tower ironing area"), "mm²"),
        unit(f("PrimeVolumeMm3", "Filament prime volume"), "mm³"),
        unit(f("RammingTravelTimeS", "Travel time after ramming"), "s"),
        unit(f("RammingTravelTimeNcS", "Travel time after ramming — High Flow"), "s"),
        numeric(unit(f("NozzleTempRangeLowC", "Recommended nozzle temperature — Min"), "°C")),
        numeric(unit(f("NozzleTempRangeHighC", "Recommended nozzle temperature — Max"), "°C")),
    ]),
    ("Filament", "Print temperature", &[
        numeric(unit(f("NozzleTempInitialC", "Nozzle — Initial layer"), "°C")),
        numeric(unit(f("NozzleTempC", "Nozzle — Other layers"), "°C")),
        unit(f("SupertackPlateTempInitialC", "Cool Plate SuperTack — Initial layer"), "°C"),
        unit(f("SupertackPlateTempC", "Cool Plate SuperTack — Other layers"), "°C"),
        unit(f("CoolPlateTempInitialC", "Cool Plate — Initial layer"), "°C"),
        unit(f("CoolPlateTempC", "Cool Plate — Other layers"), "°C"),
        unit(f("EngPlateTempInitialC", "Engineering Plate — Initial layer"), "°C"),
        unit(f("EngPlateTempC", "Engineering Plate — Other layers"), "°C"),
        unit(f("HotPlateTempInitialC", "Smooth PEI / High Temp Plate — Initial layer"), "°C"),
        unit(f("HotPlateTempC", "Smooth PEI / High Temp Plate — Other layers"), "°C"),
        unit(f("TexturedPlateTempInitialC", "Textured PEI Plate — Initial layer"), "°C"),
        unit(f("TexturedPlateTempC", "Textured PEI Plate — Other layers"), "°C"),
    ]),
    ("Filament", "Volumetric speed / scarf seam", &[
        bool_field(f("AdaptiveVolumetricSpeed", "Adaptive volumetric speed")),
        unit(f("MaxVolumetricSpeedMm3S", "Max volumetric speed"), "mm³/s"),
        unit(f("RammingVolumetricSpeedMm3S", "Ramming volumetric speed — Extruder change"), "mm³/s"),
        unit(f("RammingVolumetricSpeedNcMm3S", "Ramming volumetric speed — Hotend change"), "mm³/s"),
        options(f("ScarfSeamType", "Scarf seam type"), &["none", "external", "all"]),
        f("ScarfHeightPct", "Scarf start height"),
        f("ScarfGapPct", "Scarf slope gap"),
        unit(f("ScarfLengthMm", "Scarf length"), "mm"),
    ]),
    ("Cooling", "Part cooling fan", &[
        numeric(f("CloseFanFirstXLayers", "Initial layer fan — For the first N layers")),
        numeric(unit(f("FirstXLayerFanSpeedPct", "Initial layer fan — Fan speed"), "%")),
        numeric(unit(f("FullFanSpeedLayer", "Linear ramp up to"), "layers")),
        numeric(unit(f("FanMinSpeedPct", "Min fan speed threshold — Fan speed"), "%")),
        numeric(unit(f("FanMaxSpeedPct", "Max fan speed threshold — Fan speed"), "%")),
        numeric(unit(f("FanCoolingLayerTimeS", "Layer time"), "s")),
        bool_field(f("SlowDownForLayerCooling", "Slow printing down for better layer cooling")),
        bool_field(f("NoSlowDownForCoolingOnOutwalls", "Don't slow down outer walls")),
        options(f("CoolingSlowdownLogic", "Cooling slowdown logic"), &["uniform_cooling", "consistent_surface"]),
        unit(f("CoolingPerimeterTransitionDistanceMm", "Perimeter transition distance"), "mm"),
        unit(f("SlowDownMinSpeedMmS", "Min print speed"), "mm/s"),
        unit(f("SlowDownLayerTimeS", "Slow down layer time"), "s"),
        options(f("OverhangFanThreshold", "Cooling overhang threshold"), &["0%", "10%", "25%", "50%", "75%", "95%"]),
        options(f("OverhangThresholdParticipatingCooling", "Overhang threshold for participating cooling"), &["0%", "10%", "25%", "50%", "75%", "100%"]),
        numeric(unit(f("OverhangFanSpeedPct", "Fan speed for overhangs"), "%")),
        unit(f("PreStartFanTimeS", "Pre start fan time"), "s"),
        bool_field(f("EnableOverhangBridgeFan", "Keep fan always on")),
    ]),
    ("Cooling", "Auxiliary / exhaust", &[
        unit(f("AdditionalCoolingFanSpeedPct", "Auxiliary fan speed"), "%"),
        unit(f("DuringPrintExhaustFanSpeedPct", "During print exhaust fan speed"), "%"),
        unit(f("CompletePrintExhaustFanSpeedPct", "Complete print exhaust fan speed"), "%"),
        unit(f("ChamberTemperatureC", "Chamber temperature"), "°C"),
        bool_field(f("ActivateAirFiltration", "Activate air filtration")),
        bool_field(f("ReduceFanStopStartFreq", "Reduce fan stop/start frequency")),
    ]),
    ("Setting Overrides", "Retraction", &[
        unit(f("RetractionMm", "Length"), "mm"),
        unit(f("ZHopMm", "Z hop when retract"), "mm"),
        options(f("ZHopType", "Z Hop Type"), &["Auto Lift", "Normal Lift", "Slope Lift", "Spiral Lift"]),
        unit(f("RetractionSpeedMmS", "Retraction Speed"), "mm/s"),
        unit(f("DeretractionSpeedMmS", "Deretraction Speed"), "mm/s"),
        unit(f("ChangeLengthMm", "Length when change hotend"), "mm"),
        unit(f("RetractRestartExtraMm", "Extra length on restart"), "mm"),
        unit(f("RetractionMinimumTravelMm", "Travel distance threshold"), "mm"),
        bool_field(f("RetractWhenChangingLayer", "Retract when change layer")),
        bool_field(f("WipeEnabled", "Wipe while retracting")),
        unit(f("WipeDistanceMm", "Wipe Distance"), "mm"),
        unit(f("RetractBeforeWipe", "Retract amount before wipe"), "%"),
        bool_field(f("LongRetractionsWhenCut", "Long retraction when cut (experimental)")),
        unit(f("RetractionDistancesWhenCutMm", "Retraction distance when cut"), "mm"),
        unit(f("ChangeLengthNcMm", "Length when change hotend — High Flow"), "mm"),
        unit(f("RetractLengthNcMm", "Extra length on restart — High Flow"), "mm"),
        bool_field(f("LongRetractionsWhenEc", "Long retraction when cut — High Flow")),
        unit(f("RetractionDistancesWhenEcMm", "Retraction distance when cut — High Flow"), "mm"),
    ]),
    ("Setting Overrides", "Speed", &[
        unit(f("PrintSpeedMmS", "Print speed — manual only"), "mm/s"),
        bool_field(f("OverrideProcessOverhangSpeed", "Override overhang speed")),
        bool_field(f("EnableOverhangSpeed", "Slow down for overhangs")),
        unit(f("Overhang14SpeedMmS", "10%"), "mm/s"),
        unit(f("Overhang24SpeedMmS", "25%"), "mm/s"),
        unit(f("Overhang34SpeedMmS", "50%"), "mm/s"),
        unit(f("Overhang44SpeedMmS", "75%"), "mm/s"),
        unit(f("OverhangTotallySpeedMmS", "100%"), "mm/s"),
        unit(f("BridgeSpeedMmS", "Bridge"), "mm/s"),
    ]),
    ("Advanced", "Advanced", &[
        bool_field(f("EnablePressureAdvance", "Enable pressure advance")),
        f("PressureAdvance", "Pressure advance"),
        unit(f("CircleCompensationSpeedMmS", "Circle compensation speed"), "mm/s"),
        f("HoleCoef1", "Hole coefficient 1"),
        f("HoleCoef2", "Hole coefficient 2"),
        f("HoleCoef3", "Hole coefficient 3"),
        f("HoleLimitMax", "Hole limit max"),
        f("HoleLimitMin", "Hole limit min"),
        f("CounterCoef1", "Counter coefficient 1"),
        f("CounterCoef2", "Counter coefficient 2"),
        f("CounterCoef3", "Counter coefficient 3"),
        f("CounterLimitMax", "Counter limit max"),
        f("CounterLimitMin", "Counter limit min"),
        f("DryingAmsLimitations", "AMS drying limitations"),
        unit(f("DryingAmsHeatDistortionTempC", "AMS drying heat distortion temp"), "°C"),
        unit(f("DryingAmsTempC", "AMS drying temp"), "°C"),
        unit(f("DryingAmsTimeH", "AMS drying time"), "h"),
        unit(f("DryingChamberBedTempC", "Chamber drying bed temp"), "°C"),
        unit(f("DryingChamberTimeH", "Chamber drying time"), "h"),
        unit(f("DryingCoolingTempC", "Drying cooling temp"), "°C"),
        unit(f("DryingSofteningTempC", "Drying softening temp"), "°C"),
        unit(f("FlushTempC", "Flush temp"), "°C"),
        unit(f("FlushVolumetricSpeedMm3S", "Flush volumetric speed"), "mm³/s"),
        f("VolumetricSpeedCoefficients", "Volumetric speed coefficients"),
        text_area(f("StartGcode", "Filament start G-code")),
        text_area(f("EndGcode", "Filament end G-code")),
    ]),
    ("Notes", "Notes", &[text_area(f("SlicerNotes", "Filament notes"))]),
    ("Multi Filament", "Multi Filament", &[
        numeric(unit(f("TowerInterfacePrintTempC", "Purge temperature"), "°C")),
        unit(f("TowerInterfacePurgeVolumeMm3", "Purge volumetric speed"), "mm³/s"),
        unit(f("TowerInterfacePreExtrusionDistMm", "Tower interface pre-extrusion distance"), "mm"),
        unit(f("TowerInterfacePreExtrusionLengthMm", "Tower interface pre-extrusion length"), "mm"),
        unit(f("MinimalPurgeOnWipeTowerMm3", "Minimal purge on wipe tower"), "mm³"),
        unit(f("PrimeVolumeNcMm3", "Filament prime volume — High Flow"), "mm³"),
        unit(f("CoolingBeforeTowerS", "Cooling before tower"), "s"),
    ]),
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFieldEntry {
    name: &'static str,
    label: &'static str,
    unit: &'static str,
    is_bool: bool,
    is_text_area: bool,
    is_numeric: bool,
    options: Option<&'static [&'static str]>,
    is_enum: bool,
    is_plain_text: bool,
    hide_when_blank: bool,
    show_row: bool,
    bool_value: bool,
    value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFieldGroup {
    title: &'static str,
    fields: Vec<ProfileFieldEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFieldTab {
    title: &'static str,
    sections: Vec<ProfileFieldGroup>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFieldSpecResponse {
    pub name: String,
    pub tabs: Vec<ProfileFieldTab>,
}

pub fn build_groups(name: String, initial_values: Option<&HashMap<&'static str, String>>) -> ProfileFieldSpecResponse {
    let mut tabs: Vec<ProfileFieldTab> = Vec::new();
    for &(tab_title, section_title, fields) in GROUPS {
        let entries: Vec<ProfileFieldEntry> = fields
            .iter()
            .map(|def| {
                let value = initial_values.and_then(|m| m.get(def.name)).cloned().unwrap_or_default();
                let is_enum = def.options.is_some();
                ProfileFieldEntry {
                    name: def.name,
                    label: def.label,
                    unit: def.unit,
                    is_bool: def.is_bool,
                    is_text_area: def.is_text_area,
                    is_numeric: def.is_numeric,
                    options: def.options,
                    is_enum,
                    is_plain_text: !def.is_bool && !is_enum && !def.is_text_area,
                    hide_when_blank: def.hide_when_blank,
                    show_row: !def.hide_when_blank || !value.trim().is_empty(),
                    bool_value: value == "true",
                    value,
                }
            })
            .collect();

        match tabs.iter_mut().find(|t| t.title == tab_title) {
            Some(tab) => tab.sections.push(ProfileFieldGroup { title: section_title, fields: entries }),
            None => tabs.push(ProfileFieldTab { title: tab_title, sections: vec![ProfileFieldGroup { title: section_title, fields: entries }] }),
        }
    }

    ProfileFieldSpecResponse { name, tabs }
}

pub(crate) fn fmt_f64(v: f64) -> String {
    v.to_string()
}

// Mirrors ProfileFieldMapper.ToFieldStrings: bool -> "true"/"false", numbers via their default
// (culture-invariant-equivalent) formatting, missing/null -> "".
pub fn field_strings(p: &PrintProfile) -> HashMap<&'static str, String> {
    let mut m = HashMap::new();
    macro_rules! b {
        ($name:literal, $field:ident) => {
            m.insert($name, match p.$field { Some(true) => "true".to_string(), Some(false) => "false".to_string(), None => String::new() });
        };
    }
    macro_rules! i {
        ($name:literal, $field:ident) => {
            m.insert($name, p.$field.map(|v| v.to_string()).unwrap_or_default());
        };
    }
    macro_rules! fl {
        ($name:literal, $field:ident) => {
            m.insert($name, p.$field.map(fmt_f64).unwrap_or_default());
        };
    }
    macro_rules! s {
        ($name:literal, $field:ident) => {
            m.insert($name, p.$field.clone().unwrap_or_default());
        };
    }

    m.insert("NozzleTempC", p.nozzle_temp_c.to_string());

    i!("PrintSpeedMmS", print_speed_mm_s);
    i!("NozzleTempInitialC", nozzle_temp_initial_c);
    i!("NozzleTempRangeHighC", nozzle_temp_range_high_c);
    i!("NozzleTempRangeLowC", nozzle_temp_range_low_c);
    i!("CoolPlateTempC", cool_plate_temp_c);
    i!("CoolPlateTempInitialC", cool_plate_temp_initial_c);
    i!("HotPlateTempC", hot_plate_temp_c);
    i!("HotPlateTempInitialC", hot_plate_temp_initial_c);
    i!("TexturedPlateTempC", textured_plate_temp_c);
    i!("TexturedPlateTempInitialC", textured_plate_temp_initial_c);
    i!("EngPlateTempC", eng_plate_temp_c);
    i!("EngPlateTempInitialC", eng_plate_temp_initial_c);
    i!("SupertackPlateTempC", supertack_plate_temp_c);
    i!("SupertackPlateTempInitialC", supertack_plate_temp_initial_c);

    i!("FanMinSpeedPct", fan_min_speed_pct);
    i!("FanMaxSpeedPct", fan_max_speed_pct);
    i!("AdditionalCoolingFanSpeedPct", additional_cooling_fan_speed_pct);
    i!("CloseFanFirstXLayers", close_fan_first_x_layers);
    i!("CompletePrintExhaustFanSpeedPct", complete_print_exhaust_fan_speed_pct);
    i!("DuringPrintExhaustFanSpeedPct", during_print_exhaust_fan_speed_pct);
    i!("ChamberTemperatureC", chamber_temperature_c);
    fl!("CoolingPerimeterTransitionDistanceMm", cooling_perimeter_transition_distance_mm);
    s!("CoolingSlowdownLogic", cooling_slowdown_logic);
    b!("EnableOverhangBridgeFan", enable_overhang_bridge_fan);
    i!("FanCoolingLayerTimeS", fan_cooling_layer_time_s);
    i!("FirstXLayerFanSpeedPct", first_x_layer_fan_speed_pct);
    i!("FullFanSpeedLayer", full_fan_speed_layer);
    b!("NoSlowDownForCoolingOnOutwalls", no_slow_down_for_cooling_on_outwalls);
    i!("OverhangFanSpeedPct", overhang_fan_speed_pct);
    s!("OverhangFanThreshold", overhang_fan_threshold);
    s!("OverhangThresholdParticipatingCooling", overhang_threshold_participating_cooling);
    b!("OverrideProcessOverhangSpeed", override_process_overhang_speed);
    i!("PreStartFanTimeS", pre_start_fan_time_s);
    b!("ReduceFanStopStartFreq", reduce_fan_stop_start_freq);
    b!("SlowDownForLayerCooling", slow_down_for_layer_cooling);
    i!("SlowDownLayerTimeS", slow_down_layer_time_s);
    i!("SlowDownMinSpeedMmS", slow_down_min_speed_mm_s);
    b!("ActivateAirFiltration", activate_air_filtration);

    fl!("RetractionMm", retraction_mm);
    fl!("RetractionSpeedMmS", retraction_speed_mm_s);
    fl!("DeretractionSpeedMmS", deretraction_speed_mm_s);
    fl!("RetractionMinimumTravelMm", retraction_minimum_travel_mm);
    s!("RetractBeforeWipe", retract_before_wipe);
    fl!("RetractRestartExtraMm", retract_restart_extra_mm);
    b!("RetractWhenChangingLayer", retract_when_changing_layer);
    fl!("RetractionDistancesWhenCutMm", retraction_distances_when_cut_mm);
    fl!("RetractLengthNcMm", retract_length_nc_mm);
    b!("LongRetractionsWhenCut", long_retractions_when_cut);
    b!("LongRetractionsWhenEc", long_retractions_when_ec);
    fl!("RetractionDistancesWhenEcMm", retraction_distances_when_ec_mm);

    b!("WipeEnabled", wipe_enabled);
    fl!("WipeDistanceMm", wipe_distance_mm);
    fl!("ZHopMm", z_hop_mm);
    s!("ZHopType", z_hop_type);
    fl!("ChangeLengthMm", change_length_mm);
    fl!("ChangeLengthNcMm", change_length_nc_mm);
    i!("CoolingBeforeTowerS", cooling_before_tower_s);
    fl!("MinimalPurgeOnWipeTowerMm3", minimal_purge_on_wipe_tower_mm3);
    fl!("PrimeVolumeMm3", prime_volume_mm3);
    fl!("PrimeVolumeNcMm3", prime_volume_nc_mm3);
    fl!("RammingTravelTimeS", ramming_travel_time_s);
    fl!("RammingTravelTimeNcS", ramming_travel_time_nc_s);
    fl!("RammingVolumetricSpeedMm3S", ramming_volumetric_speed_mm3_s);
    fl!("RammingVolumetricSpeedNcMm3S", ramming_volumetric_speed_nc_mm3_s);
    fl!("TowerInterfacePreExtrusionDistMm", tower_interface_pre_extrusion_dist_mm);
    fl!("TowerInterfacePreExtrusionLengthMm", tower_interface_pre_extrusion_length_mm);
    i!("TowerInterfacePrintTempC", tower_interface_print_temp_c);
    fl!("TowerInterfacePurgeVolumeMm3", tower_interface_purge_volume_mm3);
    fl!("TowerIroningAreaMm2", tower_ironing_area_mm2);
    i!("FlushTempC", flush_temp_c);
    fl!("FlushVolumetricSpeedMm3S", flush_volumetric_speed_mm3_s);

    b!("AdaptiveVolumetricSpeed", adaptive_volumetric_speed);
    fl!("MaxVolumetricSpeedMm3S", max_volumetric_speed_mm3_s);
    fl!("BridgeSpeedMmS", bridge_speed_mm_s);
    b!("EnableOverhangSpeed", enable_overhang_speed);
    fl!("Overhang14SpeedMmS", overhang_14_speed_mm_s);
    fl!("Overhang24SpeedMmS", overhang_24_speed_mm_s);
    fl!("Overhang34SpeedMmS", overhang_34_speed_mm_s);
    fl!("Overhang44SpeedMmS", overhang_44_speed_mm_s);
    fl!("OverhangTotallySpeedMmS", overhang_totally_speed_mm_s);
    fl!("CircleCompensationSpeedMmS", circle_compensation_speed_mm_s);
    fl!("VelocityAdaptationFactor", velocity_adaptation_factor);
    s!("VolumetricSpeedCoefficients", volumetric_speed_coefficients);

    fl!("DensityGCm3", density_g_cm3);
    fl!("DiameterMm", diameter_mm);
    fl!("DiameterLimitMm", diameter_limit_mm);
    s!("ShrinkPct", shrink_pct);
    b!("Soluble", soluble);
    b!("IsSupport", is_support);
    i!("Printable", printable);
    i!("AdhesivenessCategory", adhesiveness_category);
    fl!("ImpactStrengthZ", impact_strength_z);
    fl!("CostPerKg", cost_per_kg);
    fl!("FlowRatio", flow_ratio);
    s!("ExtruderVariant", extruder_variant);
    s!("SlicerNotes", slicer_notes);
    i!("RequiredNozzleHrc", required_nozzle_hrc);

    b!("EnablePressureAdvance", enable_pressure_advance);
    fl!("PressureAdvance", pressure_advance);

    s!("DryingAmsLimitations", drying_ams_limitations);
    i!("DryingAmsHeatDistortionTempC", drying_ams_heat_distortion_temp_c);
    i!("DryingAmsTempC", drying_ams_temp_c);
    fl!("DryingAmsTimeH", drying_ams_time_h);
    i!("DryingChamberBedTempC", drying_chamber_bed_temp_c);
    fl!("DryingChamberTimeH", drying_chamber_time_h);
    i!("DryingCoolingTempC", drying_cooling_temp_c);
    i!("DryingSofteningTempC", drying_softening_temp_c);
    fl!("SofteningTempC", softening_temp_c);

    s!("ScarfSeamType", scarf_seam_type);
    s!("ScarfGapPct", scarf_gap_pct);
    s!("ScarfHeightPct", scarf_height_pct);
    fl!("ScarfLengthMm", scarf_length_mm);

    fl!("HoleCoef1", hole_coef_1);
    fl!("HoleCoef2", hole_coef_2);
    fl!("HoleCoef3", hole_coef_3);
    fl!("HoleLimitMax", hole_limit_max);
    fl!("HoleLimitMin", hole_limit_min);
    fl!("CounterCoef1", counter_coef_1);
    fl!("CounterCoef2", counter_coef_2);
    fl!("CounterCoef3", counter_coef_3);
    fl!("CounterLimitMax", counter_limit_max);
    fl!("CounterLimitMin", counter_limit_min);

    s!("StartGcode", start_gcode);
    s!("EndGcode", end_gcode);

    s!("DefaultColourHex", default_colour_hex);

    m
}

// The inverse of field_strings: string -> typed, driven by the same field list. Blank means
// "not set" (-> None), for every field except NozzleTempC (the one required column) where blank
// or unparseable is a validation error rather than a silent 0. Any other field whose non-blank
// string fails to parse is likewise collected as a field-level error instead of defaulting.
pub fn parse_fields(fields: &HashMap<String, String>) -> Result<ParsedProfileFields, Vec<(&'static str, String)>> {
    let mut errors: Vec<(&'static str, String)> = Vec::new();
    let get = |name: &str| -> &str { fields.get(name).map(String::as_str).unwrap_or("").trim() };

    macro_rules! b {
        ($name:literal) => {
            match get($name) {
                "" => None,
                "true" => Some(true),
                "false" => Some(false),
                _ => {
                    errors.push(($name, format!("{} must be true or false", $name)));
                    None
                }
            }
        };
    }
    macro_rules! i {
        ($name:literal) => {
            match get($name) {
                "" => None,
                v => match v.parse::<i64>() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        errors.push(($name, format!("{} must be a whole number", $name)));
                        None
                    }
                },
            }
        };
    }
    macro_rules! fl {
        ($name:literal) => {
            match get($name) {
                "" => None,
                v => match v.parse::<f64>() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        errors.push(($name, format!("{} must be a number", $name)));
                        None
                    }
                },
            }
        };
    }
    macro_rules! s {
        ($name:literal) => {
            match get($name) {
                "" => None,
                v => Some(v.to_string()),
            }
        };
    }

    let nozzle_temp_c = match get("NozzleTempC") {
        "" => {
            errors.push(("NozzleTempC", "Nozzle temperature is required".to_string()));
            0
        }
        v => match v.parse::<i64>() {
            Ok(n) => n,
            Err(_) => {
                errors.push(("NozzleTempC", "Nozzle temperature must be a whole number".to_string()));
                0
            }
        },
    };

    let parsed = ParsedProfileFields {
        print_speed_mm_s: i!("PrintSpeedMmS"),
        nozzle_temp_initial_c: i!("NozzleTempInitialC"),
        nozzle_temp_range_high_c: i!("NozzleTempRangeHighC"),
        nozzle_temp_range_low_c: i!("NozzleTempRangeLowC"),
        cool_plate_temp_c: i!("CoolPlateTempC"),
        cool_plate_temp_initial_c: i!("CoolPlateTempInitialC"),
        hot_plate_temp_c: i!("HotPlateTempC"),
        hot_plate_temp_initial_c: i!("HotPlateTempInitialC"),
        textured_plate_temp_c: i!("TexturedPlateTempC"),
        textured_plate_temp_initial_c: i!("TexturedPlateTempInitialC"),
        eng_plate_temp_c: i!("EngPlateTempC"),
        eng_plate_temp_initial_c: i!("EngPlateTempInitialC"),
        supertack_plate_temp_c: i!("SupertackPlateTempC"),
        supertack_plate_temp_initial_c: i!("SupertackPlateTempInitialC"),

        fan_min_speed_pct: i!("FanMinSpeedPct"),
        fan_max_speed_pct: i!("FanMaxSpeedPct"),
        additional_cooling_fan_speed_pct: i!("AdditionalCoolingFanSpeedPct"),
        close_fan_first_x_layers: i!("CloseFanFirstXLayers"),
        complete_print_exhaust_fan_speed_pct: i!("CompletePrintExhaustFanSpeedPct"),
        during_print_exhaust_fan_speed_pct: i!("DuringPrintExhaustFanSpeedPct"),
        chamber_temperature_c: i!("ChamberTemperatureC"),
        cooling_perimeter_transition_distance_mm: fl!("CoolingPerimeterTransitionDistanceMm"),
        cooling_slowdown_logic: s!("CoolingSlowdownLogic"),
        enable_overhang_bridge_fan: b!("EnableOverhangBridgeFan"),
        fan_cooling_layer_time_s: i!("FanCoolingLayerTimeS"),
        first_x_layer_fan_speed_pct: i!("FirstXLayerFanSpeedPct"),
        full_fan_speed_layer: i!("FullFanSpeedLayer"),
        no_slow_down_for_cooling_on_outwalls: b!("NoSlowDownForCoolingOnOutwalls"),
        overhang_fan_speed_pct: i!("OverhangFanSpeedPct"),
        overhang_fan_threshold: s!("OverhangFanThreshold"),
        overhang_threshold_participating_cooling: s!("OverhangThresholdParticipatingCooling"),
        override_process_overhang_speed: b!("OverrideProcessOverhangSpeed"),
        pre_start_fan_time_s: i!("PreStartFanTimeS"),
        reduce_fan_stop_start_freq: b!("ReduceFanStopStartFreq"),
        slow_down_for_layer_cooling: b!("SlowDownForLayerCooling"),
        slow_down_layer_time_s: i!("SlowDownLayerTimeS"),
        slow_down_min_speed_mm_s: i!("SlowDownMinSpeedMmS"),
        activate_air_filtration: b!("ActivateAirFiltration"),

        retraction_mm: fl!("RetractionMm"),
        retraction_speed_mm_s: fl!("RetractionSpeedMmS"),
        deretraction_speed_mm_s: fl!("DeretractionSpeedMmS"),
        retraction_minimum_travel_mm: fl!("RetractionMinimumTravelMm"),
        retract_before_wipe: s!("RetractBeforeWipe"),
        retract_restart_extra_mm: fl!("RetractRestartExtraMm"),
        retract_when_changing_layer: b!("RetractWhenChangingLayer"),
        retraction_distances_when_cut_mm: fl!("RetractionDistancesWhenCutMm"),
        retract_length_nc_mm: fl!("RetractLengthNcMm"),
        long_retractions_when_cut: b!("LongRetractionsWhenCut"),
        long_retractions_when_ec: b!("LongRetractionsWhenEc"),
        retraction_distances_when_ec_mm: fl!("RetractionDistancesWhenEcMm"),

        wipe_enabled: b!("WipeEnabled"),
        wipe_distance_mm: fl!("WipeDistanceMm"),
        z_hop_mm: fl!("ZHopMm"),
        z_hop_type: s!("ZHopType"),
        change_length_mm: fl!("ChangeLengthMm"),
        change_length_nc_mm: fl!("ChangeLengthNcMm"),
        cooling_before_tower_s: i!("CoolingBeforeTowerS"),
        minimal_purge_on_wipe_tower_mm3: fl!("MinimalPurgeOnWipeTowerMm3"),
        prime_volume_mm3: fl!("PrimeVolumeMm3"),
        prime_volume_nc_mm3: fl!("PrimeVolumeNcMm3"),
        ramming_travel_time_s: fl!("RammingTravelTimeS"),
        ramming_travel_time_nc_s: fl!("RammingTravelTimeNcS"),
        ramming_volumetric_speed_mm3_s: fl!("RammingVolumetricSpeedMm3S"),
        ramming_volumetric_speed_nc_mm3_s: fl!("RammingVolumetricSpeedNcMm3S"),
        tower_interface_pre_extrusion_dist_mm: fl!("TowerInterfacePreExtrusionDistMm"),
        tower_interface_pre_extrusion_length_mm: fl!("TowerInterfacePreExtrusionLengthMm"),
        tower_interface_print_temp_c: i!("TowerInterfacePrintTempC"),
        tower_interface_purge_volume_mm3: fl!("TowerInterfacePurgeVolumeMm3"),
        tower_ironing_area_mm2: fl!("TowerIroningAreaMm2"),
        flush_temp_c: i!("FlushTempC"),
        flush_volumetric_speed_mm3_s: fl!("FlushVolumetricSpeedMm3S"),

        adaptive_volumetric_speed: b!("AdaptiveVolumetricSpeed"),
        max_volumetric_speed_mm3_s: fl!("MaxVolumetricSpeedMm3S"),
        bridge_speed_mm_s: fl!("BridgeSpeedMmS"),
        enable_overhang_speed: b!("EnableOverhangSpeed"),
        overhang_14_speed_mm_s: fl!("Overhang14SpeedMmS"),
        overhang_24_speed_mm_s: fl!("Overhang24SpeedMmS"),
        overhang_34_speed_mm_s: fl!("Overhang34SpeedMmS"),
        overhang_44_speed_mm_s: fl!("Overhang44SpeedMmS"),
        overhang_totally_speed_mm_s: fl!("OverhangTotallySpeedMmS"),
        circle_compensation_speed_mm_s: fl!("CircleCompensationSpeedMmS"),
        velocity_adaptation_factor: fl!("VelocityAdaptationFactor"),
        volumetric_speed_coefficients: s!("VolumetricSpeedCoefficients"),

        density_g_cm3: fl!("DensityGCm3"),
        diameter_mm: fl!("DiameterMm"),
        diameter_limit_mm: fl!("DiameterLimitMm"),
        shrink_pct: s!("ShrinkPct"),
        soluble: b!("Soluble"),
        is_support: b!("IsSupport"),
        printable: i!("Printable"),
        adhesiveness_category: i!("AdhesivenessCategory"),
        impact_strength_z: fl!("ImpactStrengthZ"),
        cost_per_kg: fl!("CostPerKg"),
        flow_ratio: fl!("FlowRatio"),
        extruder_variant: s!("ExtruderVariant"),
        slicer_notes: s!("SlicerNotes"),
        required_nozzle_hrc: i!("RequiredNozzleHrc"),

        enable_pressure_advance: b!("EnablePressureAdvance"),
        pressure_advance: fl!("PressureAdvance"),

        drying_ams_limitations: s!("DryingAmsLimitations"),
        drying_ams_heat_distortion_temp_c: i!("DryingAmsHeatDistortionTempC"),
        drying_ams_temp_c: i!("DryingAmsTempC"),
        drying_ams_time_h: fl!("DryingAmsTimeH"),
        drying_chamber_bed_temp_c: i!("DryingChamberBedTempC"),
        drying_chamber_time_h: fl!("DryingChamberTimeH"),
        drying_cooling_temp_c: i!("DryingCoolingTempC"),
        drying_softening_temp_c: i!("DryingSofteningTempC"),
        softening_temp_c: fl!("SofteningTempC"),

        scarf_seam_type: s!("ScarfSeamType"),
        scarf_gap_pct: s!("ScarfGapPct"),
        scarf_height_pct: s!("ScarfHeightPct"),
        scarf_length_mm: fl!("ScarfLengthMm"),

        hole_coef_1: fl!("HoleCoef1"),
        hole_coef_2: fl!("HoleCoef2"),
        hole_coef_3: fl!("HoleCoef3"),
        hole_limit_max: fl!("HoleLimitMax"),
        hole_limit_min: fl!("HoleLimitMin"),
        counter_coef_1: fl!("CounterCoef1"),
        counter_coef_2: fl!("CounterCoef2"),
        counter_coef_3: fl!("CounterCoef3"),
        counter_limit_max: fl!("CounterLimitMax"),
        counter_limit_min: fl!("CounterLimitMin"),

        start_gcode: s!("StartGcode"),
        end_gcode: s!("EndGcode"),

        default_colour_hex: s!("DefaultColourHex"),

        nozzle_temp_c,
    };

    if errors.is_empty() { Ok(parsed) } else { Err(errors) }
}
