use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::{Json, Router, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::io::Read;

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/profiles/import-preset", post(import_preset))
        .route("/api/profiles/system-presets", get(list_system_presets))
        .route("/api/profiles/system-presets/resolve", post(resolve_system_preset))
}

// Bambu's raw JSON key -> our PrintProfile property name. Mirrors BambuFilamentImportService's
// KeyMap exactly (same key list as profile_field_spec.rs's GROUPS, translated to Bambu's own
// setting names rather than our display labels).
const KEY_MAP: &[(&str, &str)] = &[
    ("nozzle_temperature", "NozzleTempC"),
    ("nozzle_temperature_initial_layer", "NozzleTempInitialC"),
    ("nozzle_temperature_range_high", "NozzleTempRangeHighC"),
    ("nozzle_temperature_range_low", "NozzleTempRangeLowC"),
    ("cool_plate_temp", "CoolPlateTempC"),
    ("cool_plate_temp_initial_layer", "CoolPlateTempInitialC"),
    ("hot_plate_temp", "HotPlateTempC"),
    ("hot_plate_temp_initial_layer", "HotPlateTempInitialC"),
    ("textured_plate_temp", "TexturedPlateTempC"),
    ("textured_plate_temp_initial_layer", "TexturedPlateTempInitialC"),
    ("eng_plate_temp", "EngPlateTempC"),
    ("eng_plate_temp_initial_layer", "EngPlateTempInitialC"),
    ("supertack_plate_temp", "SupertackPlateTempC"),
    ("supertack_plate_temp_initial_layer", "SupertackPlateTempInitialC"),
    ("fan_min_speed", "FanMinSpeedPct"),
    ("fan_max_speed", "FanMaxSpeedPct"),
    ("additional_cooling_fan_speed", "AdditionalCoolingFanSpeedPct"),
    ("close_fan_the_first_x_layers", "CloseFanFirstXLayers"),
    ("complete_print_exhaust_fan_speed", "CompletePrintExhaustFanSpeedPct"),
    ("during_print_exhaust_fan_speed", "DuringPrintExhaustFanSpeedPct"),
    ("chamber_temperatures", "ChamberTemperatureC"),
    ("cooling_perimeter_transition_distance", "CoolingPerimeterTransitionDistanceMm"),
    ("cooling_slowdown_logic", "CoolingSlowdownLogic"),
    ("enable_overhang_bridge_fan", "EnableOverhangBridgeFan"),
    ("fan_cooling_layer_time", "FanCoolingLayerTimeS"),
    ("first_x_layer_fan_speed", "FirstXLayerFanSpeedPct"),
    ("full_fan_speed_layer", "FullFanSpeedLayer"),
    ("no_slow_down_for_cooling_on_outwalls", "NoSlowDownForCoolingOnOutwalls"),
    ("overhang_fan_speed", "OverhangFanSpeedPct"),
    ("overhang_fan_threshold", "OverhangFanThreshold"),
    ("overhang_threshold_participating_cooling", "OverhangThresholdParticipatingCooling"),
    ("override_process_overhang_speed", "OverrideProcessOverhangSpeed"),
    ("pre_start_fan_time", "PreStartFanTimeS"),
    ("reduce_fan_stop_start_freq", "ReduceFanStopStartFreq"),
    ("slow_down_for_layer_cooling", "SlowDownForLayerCooling"),
    ("slow_down_layer_time", "SlowDownLayerTimeS"),
    ("slow_down_min_speed", "SlowDownMinSpeedMmS"),
    ("activate_air_filtration", "ActivateAirFiltration"),
    ("filament_retraction_length", "RetractionMm"),
    ("filament_retraction_speed", "RetractionSpeedMmS"),
    ("filament_deretraction_speed", "DeretractionSpeedMmS"),
    ("filament_retraction_minimum_travel", "RetractionMinimumTravelMm"),
    ("filament_retract_before_wipe", "RetractBeforeWipe"),
    ("filament_retract_restart_extra", "RetractRestartExtraMm"),
    ("filament_retract_when_changing_layer", "RetractWhenChangingLayer"),
    ("filament_retraction_distances_when_cut", "RetractionDistancesWhenCutMm"),
    ("filament_retract_length_nc", "RetractLengthNcMm"),
    ("filament_long_retractions_when_cut", "LongRetractionsWhenCut"),
    ("long_retractions_when_ec", "LongRetractionsWhenEc"),
    ("retraction_distances_when_ec", "RetractionDistancesWhenEcMm"),
    ("filament_wipe", "WipeEnabled"),
    ("filament_wipe_distance", "WipeDistanceMm"),
    ("filament_z_hop", "ZHopMm"),
    ("filament_z_hop_types", "ZHopType"),
    ("filament_change_length", "ChangeLengthMm"),
    ("filament_change_length_nc", "ChangeLengthNcMm"),
    ("filament_cooling_before_tower", "CoolingBeforeTowerS"),
    ("filament_minimal_purge_on_wipe_tower", "MinimalPurgeOnWipeTowerMm3"),
    ("filament_prime_volume", "PrimeVolumeMm3"),
    ("filament_prime_volume_nc", "PrimeVolumeNcMm3"),
    ("filament_ramming_travel_time", "RammingTravelTimeS"),
    ("filament_ramming_travel_time_nc", "RammingTravelTimeNcS"),
    ("filament_ramming_volumetric_speed", "RammingVolumetricSpeedMm3S"),
    ("filament_ramming_volumetric_speed_nc", "RammingVolumetricSpeedNcMm3S"),
    ("filament_tower_interface_pre_extrusion_dist", "TowerInterfacePreExtrusionDistMm"),
    ("filament_tower_interface_pre_extrusion_length", "TowerInterfacePreExtrusionLengthMm"),
    ("filament_tower_interface_print_temp", "TowerInterfacePrintTempC"),
    ("filament_tower_interface_purge_volume", "TowerInterfacePurgeVolumeMm3"),
    ("filament_tower_ironing_area", "TowerIroningAreaMm2"),
    ("filament_flush_temp", "FlushTempC"),
    ("filament_flush_volumetric_speed", "FlushVolumetricSpeedMm3S"),
    ("filament_adaptive_volumetric_speed", "AdaptiveVolumetricSpeed"),
    ("filament_max_volumetric_speed", "MaxVolumetricSpeedMm3S"),
    ("filament_bridge_speed", "BridgeSpeedMmS"),
    ("filament_enable_overhang_speed", "EnableOverhangSpeed"),
    ("filament_overhang_1_4_speed", "Overhang14SpeedMmS"),
    ("filament_overhang_2_4_speed", "Overhang24SpeedMmS"),
    ("filament_overhang_3_4_speed", "Overhang34SpeedMmS"),
    ("filament_overhang_4_4_speed", "Overhang44SpeedMmS"),
    ("filament_overhang_totally_speed", "OverhangTotallySpeedMmS"),
    ("circle_compensation_speed", "CircleCompensationSpeedMmS"),
    ("filament_velocity_adaptation_factor", "VelocityAdaptationFactor"),
    ("volumetric_speed_coefficients", "VolumetricSpeedCoefficients"),
    ("filament_density", "DensityGCm3"),
    ("filament_diameter", "DiameterMm"),
    ("diameter_limit", "DiameterLimitMm"),
    ("filament_shrink", "ShrinkPct"),
    ("filament_soluble", "Soluble"),
    ("filament_is_support", "IsSupport"),
    ("filament_printable", "Printable"),
    ("filament_adhesiveness_category", "AdhesivenessCategory"),
    ("impact_strength_z", "ImpactStrengthZ"),
    ("filament_cost", "CostPerKg"),
    ("filament_flow_ratio", "FlowRatio"),
    ("filament_extruder_variant", "ExtruderVariant"),
    ("filament_notes", "SlicerNotes"),
    ("required_nozzle_HRC", "RequiredNozzleHrc"),
    ("enable_pressure_advance", "EnablePressureAdvance"),
    ("pressure_advance", "PressureAdvance"),
    ("filament_dev_ams_drying_ams_limitations", "DryingAmsLimitations"),
    ("filament_dev_ams_drying_heat_distortion_temperature", "DryingAmsHeatDistortionTempC"),
    ("filament_dev_ams_drying_temperature", "DryingAmsTempC"),
    ("filament_dev_ams_drying_time", "DryingAmsTimeH"),
    ("filament_dev_chamber_drying_bed_temperature", "DryingChamberBedTempC"),
    ("filament_dev_chamber_drying_time", "DryingChamberTimeH"),
    ("filament_dev_drying_cooling_temperature", "DryingCoolingTempC"),
    ("filament_dev_drying_softening_temperature", "DryingSofteningTempC"),
    ("temperature_vitrification", "SofteningTempC"),
    ("filament_scarf_seam_type", "ScarfSeamType"),
    ("filament_scarf_gap", "ScarfGapPct"),
    ("filament_scarf_height", "ScarfHeightPct"),
    ("filament_scarf_length", "ScarfLengthMm"),
    ("hole_coef_1", "HoleCoef1"),
    ("hole_coef_2", "HoleCoef2"),
    ("hole_coef_3", "HoleCoef3"),
    ("hole_limit_max", "HoleLimitMax"),
    ("hole_limit_min", "HoleLimitMin"),
    ("counter_coef_1", "CounterCoef1"),
    ("counter_coef_2", "CounterCoef2"),
    ("counter_coef_3", "CounterCoef3"),
    ("counter_limit_max", "CounterLimitMax"),
    ("counter_limit_min", "CounterLimitMin"),
    ("filament_start_gcode", "StartGcode"),
    ("filament_end_gcode", "EndGcode"),
    ("default_filament_colour", "DefaultColourHex"),
];

const BOOL_FIELDS: &[&str] = &[
    "EnableOverhangBridgeFan", "NoSlowDownForCoolingOnOutwalls", "OverrideProcessOverhangSpeed",
    "ReduceFanStopStartFreq", "SlowDownForLayerCooling", "ActivateAirFiltration",
    "RetractWhenChangingLayer", "LongRetractionsWhenCut", "LongRetractionsWhenEc", "WipeEnabled",
    "AdaptiveVolumetricSpeed", "EnableOverhangSpeed", "Soluble", "IsSupport", "EnablePressureAdvance",
];

// Bambu's raw JSON stores this with a literal "%" baked into the string (e.g. "100%"), even
// though its own editor UI strips it and shows a separate "%" unit label — our profile field
// editor already renders a unit suffix for ShrinkPct, so keeping the raw "%" would double it up.
const PERCENT_SUFFIX_FIELDS: &[&str] = &["ShrinkPct"];

fn first_value(v: &Value) -> Option<String> {
    let value = if let Some(arr) = v.as_array() { arr.first()? } else { v };
    let s = value.as_str()?;
    if s.is_empty() || s == "nil" { None } else { Some(s.to_string()) }
}

fn to_bool_string(raw: &str) -> &'static str {
    match raw {
        "1" | "true" => "true",
        _ => "false",
    }
}

fn extract_fields(merged: &serde_json::Map<String, Value>) -> serde_json::Map<String, Value> {
    let mut fields = serde_json::Map::new();
    for &(bambu_key, our_field) in KEY_MAP {
        let Some(element) = merged.get(bambu_key) else { continue };
        let Some(raw) = first_value(element) else { continue };

        let value = if BOOL_FIELDS.contains(&our_field) {
            to_bool_string(&raw).to_string()
        } else if PERCENT_SUFFIX_FIELDS.contains(&our_field) {
            raw.trim_end_matches('%').to_string()
        } else {
            raw
        };
        fields.insert(our_field.to_string(), Value::String(value));
    }
    fields
}

// Reads an uploaded .3mf's baked Metadata/project_settings.config directly — Bambu Studio
// resolves the whole inherits chain at slice time, so unlike the (unported, Desktop-only)
// per-preset-file import, there's no chain left to walk.
fn import_from_three_mf(bytes: &[u8]) -> Result<(serde_json::Map<String, Value>, String), &'static str> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|_| "invalid_3mf")?;
    let mut entry = archive.by_name("Metadata/project_settings.config").map_err(|_| "no_project_settings")?;

    let mut json = String::new();
    entry.read_to_string(&mut json).map_err(|_| "invalid_3mf")?;
    drop(entry);

    let parsed: Value = serde_json::from_str(&json).map_err(|_| "invalid_json")?;
    let merged = parsed.as_object().ok_or("invalid_json")?.clone();
    Ok((merged, json))
}

// Accepts either a sliced .3mf (Bambu bakes the fully-resolved chain into
// Metadata/project_settings.config at slice time -- nothing left to walk) or a raw Bambu Studio
// preset export (.json, still carrying its own `inherits` chain, resolved below against the
// system library -- github.com/t2vi/spoolbook/issues/99). One input, format auto-detected: only
// falls through to the raw-JSON attempt when the upload isn't a zip at all, so a real .3mf with a
// missing config entry still reports that specific error rather than a confusing JSON one.
async fn import_preset(_editor: crate::auth::Editor, mut multipart: Multipart) -> (StatusCode, Json<Value>) {
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(field)) if field.name() == Some("file") => break Some(field),
            Ok(Some(_)) => continue,
            Ok(None) => break None,
            Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Expected multipart form data." }))),
        }
    };
    let Some(field) = field else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No file provided." })));
    };

    let file_name = field.file_name().unwrap_or("upload.3mf").to_string();
    let suggested_name_from_filename =
        file_name.trim_end_matches(".3mf").trim_end_matches(".3MF").trim_end_matches(".json").trim_end_matches(".JSON").to_string();

    let bytes = match field.bytes().await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Couldn't read the uploaded file." }))),
    };

    match import_from_three_mf(&bytes) {
        Ok((merged, raw_settings_json)) => {
            let fields = extract_fields(&merged);
            (StatusCode::OK, Json(json!({ "ok": true, "suggestedName": suggested_name_from_filename, "fields": fields, "rawSettingsJson": raw_settings_json })))
        }
        Err("invalid_3mf") => {
            let Some(leaf) = std::str::from_utf8(&bytes).ok().and_then(|s| serde_json::from_str::<Value>(s).ok()).and_then(|v| v.as_object().cloned()) else {
                return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "invalid_file" })));
            };
            let suggested_name = leaf.get("name").and_then(|v| v.as_str()).map(str::to_string).unwrap_or(suggested_name_from_filename);
            match resolve_inherits_chain(leaf).await {
                Ok(merged) => {
                    let fields = extract_fields(&merged);
                    let raw_settings_json = serde_json::to_string(&merged).unwrap_or_default();
                    (StatusCode::OK, Json(json!({ "ok": true, "suggestedName": suggested_name, "fields": fields, "rawSettingsJson": raw_settings_json })))
                }
                Err(error) => (StatusCode::BAD_GATEWAY, Json(json!({ "ok": false, "error": error }))),
            }
        }
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": error }))),
    }
}

fn slicer_base_url() -> String {
    std::env::var("RESLICE_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8100".to_string())
}

async fn fetch_json(url: &str) -> Result<Value, String> {
    let resp = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("request to slicer-service failed: {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

async fn fetch_bbl_manifest() -> Result<Value, String> {
    fetch_json(&format!("{}/profiles/BBL.json", slicer_base_url())).await
}

fn bbl_filament_list(manifest: &Value) -> Vec<&Value> {
    manifest.get("filament_list").and_then(|v| v.as_array()).map(|a| a.iter().collect()).unwrap_or_default()
}

async fn fetch_system_preset_raw(name: &str) -> Result<Map<String, Value>, String> {
    let manifest = fetch_bbl_manifest().await?;
    let sub_path = bbl_filament_list(&manifest)
        .iter()
        .find(|e| e.get("name").and_then(|n| n.as_str()) == Some(name))
        .and_then(|e| e.get("sub_path"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("system preset not found: {name}"))?;
    let url = format!("{}/profiles/BBL/{}", slicer_base_url(), sub_path);
    fetch_json(&url).await?.as_object().cloned().ok_or_else(|| "bad preset json from slicer-service".to_string())
}

// Walks `inherits` upward against the system library, child values winning over ancestors --
// mirrors the retired .NET BambuPresetResolver.ResolveAsync exactly, just fetching each ancestor
// over HTTP (slicer-service's static mount) instead of reading local disk. `leaf` is either an
// uploaded preset's own parsed object, or a `{"inherits": <name>}` stub for browsing a system
// preset directly (same stub trick bambuddy's own equivalent resolver uses for that case).
async fn resolve_inherits_chain(leaf: Map<String, Value>) -> Result<Map<String, Value>, String> {
    let mut next = leaf.get("inherits").and_then(|v| v.as_str()).map(str::to_string);
    let mut merged = leaf;
    let mut visited = HashSet::new();
    while let Some(name) = next {
        if !visited.insert(name.clone()) {
            break;
        }
        let parent = fetch_system_preset_raw(&name).await?;
        next = parent.get("inherits").and_then(|v| v.as_str()).map(str::to_string);
        for (k, v) in parent {
            merged.entry(k).or_insert(v);
        }
    }
    Ok(merged)
}

async fn list_system_presets() -> (StatusCode, Json<Value>) {
    match fetch_bbl_manifest().await {
        Ok(manifest) => {
            let mut names: Vec<String> =
                bbl_filament_list(&manifest).iter().filter_map(|e| e.get("name").and_then(|n| n.as_str()).map(str::to_string)).collect();
            names.sort();
            names.dedup();
            (StatusCode::OK, Json(json!({ "ok": true, "names": names })))
        }
        Err(error) => (StatusCode::BAD_GATEWAY, Json(json!({ "ok": false, "error": error }))),
    }
}

#[derive(Deserialize)]
struct ResolveSystemPresetRequest {
    name: String,
}

async fn resolve_system_preset(_editor: crate::auth::Editor, Json(req): Json<ResolveSystemPresetRequest>) -> (StatusCode, Json<Value>) {
    let mut leaf = Map::new();
    leaf.insert("inherits".to_string(), Value::String(req.name.clone()));
    match resolve_inherits_chain(leaf).await {
        Ok(merged) => {
            let fields = extract_fields(&merged);
            (StatusCode::OK, Json(json!({ "ok": true, "suggestedName": req.name, "fields": fields })))
        }
        Err(error) => (StatusCode::BAD_GATEWAY, Json(json!({ "ok": false, "error": error }))),
    }
}
