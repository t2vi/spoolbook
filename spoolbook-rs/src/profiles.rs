use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

// Deliberate wire-contract deviation from .NET: the C# API accepts `fields: Record<string,
// string>` because .NET's reflection-based ProfileFieldMapper needs a uniform string type to
// avoid hand-writing this ~125-field list three times (see PR notes / ADR-0026 discussion).
// Rust has no reflection, but serde's derive gets the same DRY win for free on a properly typed
// struct — so this uses real JSON types (numbers, booleans) instead. Nothing currently calls
// this from Svelte yet (ADR-0026: isolated dev, no live coexistence), so there's no live
// contract to break; whenever Profiles' Svelte page is pointed at this backend, its form will
// need to send typed values instead of stringified ones — a real, separate future step.
#[derive(Serialize, sqlx::FromRow, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrintProfile {
    pub id: i64,
    pub filament_id: i64,
    pub spool_id: Option<i64>,
    pub name: String,
    pub print_speed_mm_s: Option<i64>,

    pub nozzle_temp_c: i64,
    pub nozzle_temp_initial_c: Option<i64>,
    pub nozzle_temp_range_high_c: Option<i64>,
    pub nozzle_temp_range_low_c: Option<i64>,
    pub cool_plate_temp_c: Option<i64>,
    pub cool_plate_temp_initial_c: Option<i64>,
    pub hot_plate_temp_c: Option<i64>,
    pub hot_plate_temp_initial_c: Option<i64>,
    pub textured_plate_temp_c: Option<i64>,
    pub textured_plate_temp_initial_c: Option<i64>,
    pub eng_plate_temp_c: Option<i64>,
    pub eng_plate_temp_initial_c: Option<i64>,
    pub supertack_plate_temp_c: Option<i64>,
    pub supertack_plate_temp_initial_c: Option<i64>,

    pub fan_min_speed_pct: Option<i64>,
    pub fan_max_speed_pct: Option<i64>,
    pub additional_cooling_fan_speed_pct: Option<i64>,
    pub close_fan_first_x_layers: Option<i64>,
    pub complete_print_exhaust_fan_speed_pct: Option<i64>,
    pub during_print_exhaust_fan_speed_pct: Option<i64>,
    pub chamber_temperature_c: Option<i64>,
    pub cooling_perimeter_transition_distance_mm: Option<f64>,
    pub cooling_slowdown_logic: Option<String>,
    pub enable_overhang_bridge_fan: Option<bool>,
    pub fan_cooling_layer_time_s: Option<i64>,
    pub first_x_layer_fan_speed_pct: Option<i64>,
    pub full_fan_speed_layer: Option<i64>,
    pub no_slow_down_for_cooling_on_outwalls: Option<bool>,
    pub overhang_fan_speed_pct: Option<i64>,
    pub overhang_fan_threshold: Option<String>,
    pub overhang_threshold_participating_cooling: Option<String>,
    pub override_process_overhang_speed: Option<bool>,
    pub pre_start_fan_time_s: Option<i64>,
    pub reduce_fan_stop_start_freq: Option<bool>,
    pub slow_down_for_layer_cooling: Option<bool>,
    pub slow_down_layer_time_s: Option<i64>,
    pub slow_down_min_speed_mm_s: Option<i64>,
    pub activate_air_filtration: Option<bool>,

    pub retraction_mm: Option<f64>,
    pub retraction_speed_mm_s: Option<f64>,
    pub deretraction_speed_mm_s: Option<f64>,
    pub retraction_minimum_travel_mm: Option<f64>,
    pub retract_before_wipe: Option<String>,
    pub retract_restart_extra_mm: Option<f64>,
    pub retract_when_changing_layer: Option<bool>,
    pub retraction_distances_when_cut_mm: Option<f64>,
    pub retract_length_nc_mm: Option<f64>,
    pub long_retractions_when_cut: Option<bool>,
    pub long_retractions_when_ec: Option<bool>,
    pub retraction_distances_when_ec_mm: Option<f64>,

    pub wipe_enabled: Option<bool>,
    pub wipe_distance_mm: Option<f64>,
    pub z_hop_mm: Option<f64>,
    pub z_hop_type: Option<String>,
    pub change_length_mm: Option<f64>,
    pub change_length_nc_mm: Option<f64>,
    pub cooling_before_tower_s: Option<i64>,
    pub minimal_purge_on_wipe_tower_mm3: Option<f64>,
    pub prime_volume_mm3: Option<f64>,
    pub prime_volume_nc_mm3: Option<f64>,
    pub ramming_travel_time_s: Option<f64>,
    pub ramming_travel_time_nc_s: Option<f64>,
    pub ramming_volumetric_speed_mm3_s: Option<f64>,
    pub ramming_volumetric_speed_nc_mm3_s: Option<f64>,
    pub tower_interface_pre_extrusion_dist_mm: Option<f64>,
    pub tower_interface_pre_extrusion_length_mm: Option<f64>,
    pub tower_interface_print_temp_c: Option<i64>,
    pub tower_interface_purge_volume_mm3: Option<f64>,
    pub tower_ironing_area_mm2: Option<f64>,
    pub flush_temp_c: Option<i64>,
    pub flush_volumetric_speed_mm3_s: Option<f64>,

    pub adaptive_volumetric_speed: Option<bool>,
    pub max_volumetric_speed_mm3_s: Option<f64>,
    pub bridge_speed_mm_s: Option<f64>,
    pub enable_overhang_speed: Option<bool>,
    pub overhang_14_speed_mm_s: Option<f64>,
    pub overhang_24_speed_mm_s: Option<f64>,
    pub overhang_34_speed_mm_s: Option<f64>,
    pub overhang_44_speed_mm_s: Option<f64>,
    pub overhang_totally_speed_mm_s: Option<f64>,
    pub circle_compensation_speed_mm_s: Option<f64>,
    pub velocity_adaptation_factor: Option<f64>,
    pub volumetric_speed_coefficients: Option<String>,

    pub density_g_cm3: Option<f64>,
    pub diameter_mm: Option<f64>,
    pub diameter_limit_mm: Option<f64>,
    pub shrink_pct: Option<String>,
    pub soluble: Option<bool>,
    pub is_support: Option<bool>,
    pub printable: Option<i64>,
    pub adhesiveness_category: Option<i64>,
    pub impact_strength_z: Option<f64>,
    pub cost_per_kg: Option<f64>,
    pub flow_ratio: Option<f64>,
    pub extruder_variant: Option<String>,
    pub slicer_notes: Option<String>,
    pub required_nozzle_hrc: Option<i64>,

    pub enable_pressure_advance: Option<bool>,
    pub pressure_advance: Option<f64>,

    pub drying_ams_limitations: Option<String>,
    pub drying_ams_heat_distortion_temp_c: Option<i64>,
    pub drying_ams_temp_c: Option<i64>,
    pub drying_ams_time_h: Option<f64>,
    pub drying_chamber_bed_temp_c: Option<i64>,
    pub drying_chamber_time_h: Option<f64>,
    pub drying_cooling_temp_c: Option<i64>,
    pub drying_softening_temp_c: Option<i64>,
    pub softening_temp_c: Option<f64>,

    pub scarf_seam_type: Option<String>,
    pub scarf_gap_pct: Option<String>,
    pub scarf_height_pct: Option<String>,
    pub scarf_length_mm: Option<f64>,

    pub hole_coef_1: Option<f64>,
    pub hole_coef_2: Option<f64>,
    pub hole_coef_3: Option<f64>,
    pub hole_limit_max: Option<f64>,
    pub hole_limit_min: Option<f64>,
    pub counter_coef_1: Option<f64>,
    pub counter_coef_2: Option<f64>,
    pub counter_coef_3: Option<f64>,
    pub counter_limit_max: Option<f64>,
    pub counter_limit_min: Option<f64>,

    pub start_gcode: Option<String>,
    pub end_gcode: Option<String>,

    pub default_colour_hex: Option<String>,

    pub source: String,
    pub source_slicer: Option<String>,
    pub raw_settings_json: Option<String>,
    pub source_preset_path: Option<String>,
    pub version_number: i64,
    pub version_name: Option<String>,
    pub is_current_version: bool,
    pub notes: Option<String>,
}

// Same field set as PrintProfile, minus id/is_current_version/created_at (server-assigned) and
// filament_id (query param on create, immutable after — matches .NET, which never lets a
// profile move to a different filament).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub spool_id: Option<i64>,
    pub name: String,
    pub print_speed_mm_s: Option<i64>,

    pub nozzle_temp_c: i64,
    pub nozzle_temp_initial_c: Option<i64>,
    pub nozzle_temp_range_high_c: Option<i64>,
    pub nozzle_temp_range_low_c: Option<i64>,
    pub cool_plate_temp_c: Option<i64>,
    pub cool_plate_temp_initial_c: Option<i64>,
    pub hot_plate_temp_c: Option<i64>,
    pub hot_plate_temp_initial_c: Option<i64>,
    pub textured_plate_temp_c: Option<i64>,
    pub textured_plate_temp_initial_c: Option<i64>,
    pub eng_plate_temp_c: Option<i64>,
    pub eng_plate_temp_initial_c: Option<i64>,
    pub supertack_plate_temp_c: Option<i64>,
    pub supertack_plate_temp_initial_c: Option<i64>,

    pub fan_min_speed_pct: Option<i64>,
    pub fan_max_speed_pct: Option<i64>,
    pub additional_cooling_fan_speed_pct: Option<i64>,
    pub close_fan_first_x_layers: Option<i64>,
    pub complete_print_exhaust_fan_speed_pct: Option<i64>,
    pub during_print_exhaust_fan_speed_pct: Option<i64>,
    pub chamber_temperature_c: Option<i64>,
    pub cooling_perimeter_transition_distance_mm: Option<f64>,
    pub cooling_slowdown_logic: Option<String>,
    pub enable_overhang_bridge_fan: Option<bool>,
    pub fan_cooling_layer_time_s: Option<i64>,
    pub first_x_layer_fan_speed_pct: Option<i64>,
    pub full_fan_speed_layer: Option<i64>,
    pub no_slow_down_for_cooling_on_outwalls: Option<bool>,
    pub overhang_fan_speed_pct: Option<i64>,
    pub overhang_fan_threshold: Option<String>,
    pub overhang_threshold_participating_cooling: Option<String>,
    pub override_process_overhang_speed: Option<bool>,
    pub pre_start_fan_time_s: Option<i64>,
    pub reduce_fan_stop_start_freq: Option<bool>,
    pub slow_down_for_layer_cooling: Option<bool>,
    pub slow_down_layer_time_s: Option<i64>,
    pub slow_down_min_speed_mm_s: Option<i64>,
    pub activate_air_filtration: Option<bool>,

    pub retraction_mm: Option<f64>,
    pub retraction_speed_mm_s: Option<f64>,
    pub deretraction_speed_mm_s: Option<f64>,
    pub retraction_minimum_travel_mm: Option<f64>,
    pub retract_before_wipe: Option<String>,
    pub retract_restart_extra_mm: Option<f64>,
    pub retract_when_changing_layer: Option<bool>,
    pub retraction_distances_when_cut_mm: Option<f64>,
    pub retract_length_nc_mm: Option<f64>,
    pub long_retractions_when_cut: Option<bool>,
    pub long_retractions_when_ec: Option<bool>,
    pub retraction_distances_when_ec_mm: Option<f64>,

    pub wipe_enabled: Option<bool>,
    pub wipe_distance_mm: Option<f64>,
    pub z_hop_mm: Option<f64>,
    pub z_hop_type: Option<String>,
    pub change_length_mm: Option<f64>,
    pub change_length_nc_mm: Option<f64>,
    pub cooling_before_tower_s: Option<i64>,
    pub minimal_purge_on_wipe_tower_mm3: Option<f64>,
    pub prime_volume_mm3: Option<f64>,
    pub prime_volume_nc_mm3: Option<f64>,
    pub ramming_travel_time_s: Option<f64>,
    pub ramming_travel_time_nc_s: Option<f64>,
    pub ramming_volumetric_speed_mm3_s: Option<f64>,
    pub ramming_volumetric_speed_nc_mm3_s: Option<f64>,
    pub tower_interface_pre_extrusion_dist_mm: Option<f64>,
    pub tower_interface_pre_extrusion_length_mm: Option<f64>,
    pub tower_interface_print_temp_c: Option<i64>,
    pub tower_interface_purge_volume_mm3: Option<f64>,
    pub tower_ironing_area_mm2: Option<f64>,
    pub flush_temp_c: Option<i64>,
    pub flush_volumetric_speed_mm3_s: Option<f64>,

    pub adaptive_volumetric_speed: Option<bool>,
    pub max_volumetric_speed_mm3_s: Option<f64>,
    pub bridge_speed_mm_s: Option<f64>,
    pub enable_overhang_speed: Option<bool>,
    pub overhang_14_speed_mm_s: Option<f64>,
    pub overhang_24_speed_mm_s: Option<f64>,
    pub overhang_34_speed_mm_s: Option<f64>,
    pub overhang_44_speed_mm_s: Option<f64>,
    pub overhang_totally_speed_mm_s: Option<f64>,
    pub circle_compensation_speed_mm_s: Option<f64>,
    pub velocity_adaptation_factor: Option<f64>,
    pub volumetric_speed_coefficients: Option<String>,

    pub density_g_cm3: Option<f64>,
    pub diameter_mm: Option<f64>,
    pub diameter_limit_mm: Option<f64>,
    pub shrink_pct: Option<String>,
    pub soluble: Option<bool>,
    pub is_support: Option<bool>,
    pub printable: Option<i64>,
    pub adhesiveness_category: Option<i64>,
    pub impact_strength_z: Option<f64>,
    pub cost_per_kg: Option<f64>,
    pub flow_ratio: Option<f64>,
    pub extruder_variant: Option<String>,
    pub slicer_notes: Option<String>,
    pub required_nozzle_hrc: Option<i64>,

    pub enable_pressure_advance: Option<bool>,
    pub pressure_advance: Option<f64>,

    pub drying_ams_limitations: Option<String>,
    pub drying_ams_heat_distortion_temp_c: Option<i64>,
    pub drying_ams_temp_c: Option<i64>,
    pub drying_ams_time_h: Option<f64>,
    pub drying_chamber_bed_temp_c: Option<i64>,
    pub drying_chamber_time_h: Option<f64>,
    pub drying_cooling_temp_c: Option<i64>,
    pub drying_softening_temp_c: Option<i64>,
    pub softening_temp_c: Option<f64>,

    pub scarf_seam_type: Option<String>,
    pub scarf_gap_pct: Option<String>,
    pub scarf_height_pct: Option<String>,
    pub scarf_length_mm: Option<f64>,

    pub hole_coef_1: Option<f64>,
    pub hole_coef_2: Option<f64>,
    pub hole_coef_3: Option<f64>,
    pub hole_limit_max: Option<f64>,
    pub hole_limit_min: Option<f64>,
    pub counter_coef_1: Option<f64>,
    pub counter_coef_2: Option<f64>,
    pub counter_coef_3: Option<f64>,
    pub counter_limit_max: Option<f64>,
    pub counter_limit_min: Option<f64>,

    pub start_gcode: Option<String>,
    pub end_gcode: Option<String>,

    pub default_colour_hex: Option<String>,

    pub source: Option<String>,
    pub source_slicer: Option<String>,
    pub raw_settings_json: Option<String>,
    pub source_preset_path: Option<String>,
    pub version_name: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<PrintProfile>,
}

#[derive(Deserialize)]
pub struct FilamentIdQuery {
    #[serde(rename = "filamentId")]
    filament_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInventoryResult {
    profiles: Vec<PrintProfile>,
    total: i64,
    page: i64,
    page_size: i64,
    total_pages: i64,
}

pub(crate) async fn get_by_id(pool: &SqlitePool, id: i64) -> Option<PrintProfile> {
    let sql = format!("SELECT {COLUMNS} FROM print_profiles WHERE id = ?1");
    sqlx::query_as::<_, PrintProfile>(&sql).bind(id).fetch_optional(pool).await.expect("query failed")
}

const COLUMNS: &str = "id, filament_id, spool_id, name, print_speed_mm_s,
    nozzle_temp_c, nozzle_temp_initial_c, nozzle_temp_range_high_c, nozzle_temp_range_low_c,
    cool_plate_temp_c, cool_plate_temp_initial_c, hot_plate_temp_c, hot_plate_temp_initial_c,
    textured_plate_temp_c, textured_plate_temp_initial_c, eng_plate_temp_c, eng_plate_temp_initial_c,
    supertack_plate_temp_c, supertack_plate_temp_initial_c,
    fan_min_speed_pct, fan_max_speed_pct, additional_cooling_fan_speed_pct, close_fan_first_x_layers,
    complete_print_exhaust_fan_speed_pct, during_print_exhaust_fan_speed_pct, chamber_temperature_c,
    cooling_perimeter_transition_distance_mm, cooling_slowdown_logic, enable_overhang_bridge_fan,
    fan_cooling_layer_time_s, first_x_layer_fan_speed_pct, full_fan_speed_layer,
    no_slow_down_for_cooling_on_outwalls, overhang_fan_speed_pct, overhang_fan_threshold,
    overhang_threshold_participating_cooling, override_process_overhang_speed, pre_start_fan_time_s,
    reduce_fan_stop_start_freq, slow_down_for_layer_cooling, slow_down_layer_time_s,
    slow_down_min_speed_mm_s, activate_air_filtration,
    retraction_mm, retraction_speed_mm_s, deretraction_speed_mm_s, retraction_minimum_travel_mm,
    retract_before_wipe, retract_restart_extra_mm, retract_when_changing_layer,
    retraction_distances_when_cut_mm, retract_length_nc_mm, long_retractions_when_cut,
    long_retractions_when_ec, retraction_distances_when_ec_mm,
    wipe_enabled, wipe_distance_mm, z_hop_mm, z_hop_type, change_length_mm, change_length_nc_mm,
    cooling_before_tower_s, minimal_purge_on_wipe_tower_mm3, prime_volume_mm3, prime_volume_nc_mm3,
    ramming_travel_time_s, ramming_travel_time_nc_s, ramming_volumetric_speed_mm3_s,
    ramming_volumetric_speed_nc_mm3_s, tower_interface_pre_extrusion_dist_mm,
    tower_interface_pre_extrusion_length_mm, tower_interface_print_temp_c,
    tower_interface_purge_volume_mm3, tower_ironing_area_mm2, flush_temp_c, flush_volumetric_speed_mm3_s,
    adaptive_volumetric_speed, max_volumetric_speed_mm3_s, bridge_speed_mm_s, enable_overhang_speed,
    overhang_14_speed_mm_s, overhang_24_speed_mm_s, overhang_34_speed_mm_s, overhang_44_speed_mm_s,
    overhang_totally_speed_mm_s, circle_compensation_speed_mm_s, velocity_adaptation_factor,
    volumetric_speed_coefficients,
    density_g_cm3, diameter_mm, diameter_limit_mm, shrink_pct, soluble, is_support, printable,
    adhesiveness_category, impact_strength_z, cost_per_kg, flow_ratio, extruder_variant, slicer_notes,
    required_nozzle_hrc,
    enable_pressure_advance, pressure_advance,
    drying_ams_limitations, drying_ams_heat_distortion_temp_c, drying_ams_temp_c, drying_ams_time_h,
    drying_chamber_bed_temp_c, drying_chamber_time_h, drying_cooling_temp_c, drying_softening_temp_c,
    softening_temp_c,
    scarf_seam_type, scarf_gap_pct, scarf_height_pct, scarf_length_mm,
    hole_coef_1, hole_coef_2, hole_coef_3, hole_limit_max, hole_limit_min,
    counter_coef_1, counter_coef_2, counter_coef_3, counter_limit_max, counter_limit_min,
    start_gcode, end_gcode, default_colour_hex,
    source, source_slicer, raw_settings_json, source_preset_path, version_number, version_name,
    is_current_version, notes";

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/api/profiles", get(list_for_filament).post(create))
        .route("/api/profiles/inventory", get(inventory))
        .route("/api/profiles/field-spec", get(field_spec))
        .route("/api/profiles/{id}", axum::routing::put(update).delete(delete))
}

#[derive(Deserialize)]
struct FieldSpecQuery {
    #[serde(rename = "profileId")]
    profile_id: Option<i64>,
}

// Serves both /profiles/new (no profileId — blank tabs) and /profiles/edit/{id} (tabs
// pre-filled from the existing profile's fields), matching ProfileEndpoints.cs's single
// field-spec route.
async fn field_spec(
    State(pool): State<SqlitePool>,
    Query(q): Query<FieldSpecQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let Some(profile_id) = q.profile_id else {
        let spec = crate::profile_field_spec::build_groups(String::new(), None);
        return (StatusCode::OK, Json(serde_json::to_value(spec).unwrap()));
    };

    let sql = format!("SELECT {COLUMNS} FROM print_profiles WHERE id = ?1");
    let profile = sqlx::query_as::<_, PrintProfile>(&sql)
        .bind(profile_id)
        .fetch_optional(&pool)
        .await
        .expect("query failed");

    match profile {
        Some(profile) => {
            let values = crate::profile_field_spec::field_strings(&profile);
            let spec = crate::profile_field_spec::build_groups(profile.name.clone(), Some(&values));
            (StatusCode::OK, Json(serde_json::to_value(spec).unwrap()))
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not_found" }))),
    }
}

async fn list_for_filament(
    State(pool): State<SqlitePool>,
    Query(q): Query<FilamentIdQuery>,
) -> Json<Vec<PrintProfile>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM print_profiles WHERE filament_id = ?1 AND is_current_version = 1
         ORDER BY (spool_id IS NOT NULL), name"
    );
    let profiles = sqlx::query_as::<_, PrintProfile>(&sql)
        .bind(q.filament_id)
        .fetch_all(&pool)
        .await
        .expect("query failed");

    Json(profiles)
}

async fn inventory(State(pool): State<SqlitePool>) -> Json<ProfileInventoryResult> {
    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM print_profiles WHERE is_current_version = 1")
        .fetch_one(&pool)
        .await
        .expect("query failed");

    let sql = format!("SELECT {COLUMNS} FROM print_profiles WHERE is_current_version = 1 ORDER BY name");
    let profiles = sqlx::query_as::<_, PrintProfile>(&sql)
        .fetch_all(&pool)
        .await
        .expect("query failed");

    // No pagination on this endpoint yet (matches .NET's ProfileInventoryService — small
    // personal catalog, page/pageSize are reported but not yet enforced).
    let page_size = total.max(1);
    Json(ProfileInventoryResult { profiles, total, page: 1, page_size, total_pages: 1 })
}

fn validation_error(field: &str, message: &str) -> (StatusCode, Json<ProfileResult>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ProfileResult {
            ok: false,
            errors: Some(serde_json::json!({ field: message })),
            profile: None,
        }),
    )
}

async fn create(
    State(pool): State<SqlitePool>,
    Query(q): Query<FilamentIdQuery>,
    Json(input): Json<ProfileInput>,
) -> (StatusCode, Json<ProfileResult>) {
    if input.name.trim().is_empty() {
        return validation_error("name", "Name is required");
    }

    let sql = format!(
        "INSERT INTO print_profiles (
            filament_id, spool_id, name, print_speed_mm_s,
            nozzle_temp_c, nozzle_temp_initial_c, nozzle_temp_range_high_c, nozzle_temp_range_low_c,
            cool_plate_temp_c, cool_plate_temp_initial_c, hot_plate_temp_c, hot_plate_temp_initial_c,
            textured_plate_temp_c, textured_plate_temp_initial_c, eng_plate_temp_c, eng_plate_temp_initial_c,
            supertack_plate_temp_c, supertack_plate_temp_initial_c,
            fan_min_speed_pct, fan_max_speed_pct, additional_cooling_fan_speed_pct, close_fan_first_x_layers,
            complete_print_exhaust_fan_speed_pct, during_print_exhaust_fan_speed_pct, chamber_temperature_c,
            cooling_perimeter_transition_distance_mm, cooling_slowdown_logic, enable_overhang_bridge_fan,
            fan_cooling_layer_time_s, first_x_layer_fan_speed_pct, full_fan_speed_layer,
            no_slow_down_for_cooling_on_outwalls, overhang_fan_speed_pct, overhang_fan_threshold,
            overhang_threshold_participating_cooling, override_process_overhang_speed, pre_start_fan_time_s,
            reduce_fan_stop_start_freq, slow_down_for_layer_cooling, slow_down_layer_time_s,
            slow_down_min_speed_mm_s, activate_air_filtration,
            retraction_mm, retraction_speed_mm_s, deretraction_speed_mm_s, retraction_minimum_travel_mm,
            retract_before_wipe, retract_restart_extra_mm, retract_when_changing_layer,
            retraction_distances_when_cut_mm, retract_length_nc_mm, long_retractions_when_cut,
            long_retractions_when_ec, retraction_distances_when_ec_mm,
            wipe_enabled, wipe_distance_mm, z_hop_mm, z_hop_type, change_length_mm, change_length_nc_mm,
            cooling_before_tower_s, minimal_purge_on_wipe_tower_mm3, prime_volume_mm3, prime_volume_nc_mm3,
            ramming_travel_time_s, ramming_travel_time_nc_s, ramming_volumetric_speed_mm3_s,
            ramming_volumetric_speed_nc_mm3_s, tower_interface_pre_extrusion_dist_mm,
            tower_interface_pre_extrusion_length_mm, tower_interface_print_temp_c,
            tower_interface_purge_volume_mm3, tower_ironing_area_mm2, flush_temp_c, flush_volumetric_speed_mm3_s,
            adaptive_volumetric_speed, max_volumetric_speed_mm3_s, bridge_speed_mm_s, enable_overhang_speed,
            overhang_14_speed_mm_s, overhang_24_speed_mm_s, overhang_34_speed_mm_s, overhang_44_speed_mm_s,
            overhang_totally_speed_mm_s, circle_compensation_speed_mm_s, velocity_adaptation_factor,
            volumetric_speed_coefficients,
            density_g_cm3, diameter_mm, diameter_limit_mm, shrink_pct, soluble, is_support, printable,
            adhesiveness_category, impact_strength_z, cost_per_kg, flow_ratio, extruder_variant, slicer_notes,
            required_nozzle_hrc,
            enable_pressure_advance, pressure_advance,
            drying_ams_limitations, drying_ams_heat_distortion_temp_c, drying_ams_temp_c, drying_ams_time_h,
            drying_chamber_bed_temp_c, drying_chamber_time_h, drying_cooling_temp_c, drying_softening_temp_c,
            softening_temp_c,
            scarf_seam_type, scarf_gap_pct, scarf_height_pct, scarf_length_mm,
            hole_coef_1, hole_coef_2, hole_coef_3, hole_limit_max, hole_limit_min,
            counter_coef_1, counter_coef_2, counter_coef_3, counter_limit_max, counter_limit_min,
            start_gcode, end_gcode, default_colour_hex,
            source, source_slicer, raw_settings_json, source_preset_path, version_name, notes
        ) VALUES ({placeholders})
        RETURNING {COLUMNS}",
        placeholders = std::iter::repeat_n("?", 135).collect::<Vec<_>>().join(", "),
    );

    let profile = sqlx::query_as::<_, PrintProfile>(&sql)
        .bind(q.filament_id)
        .bind(input.spool_id)
        .bind(&input.name)
        .bind(input.print_speed_mm_s)
        .bind(input.nozzle_temp_c)
        .bind(input.nozzle_temp_initial_c)
        .bind(input.nozzle_temp_range_high_c)
        .bind(input.nozzle_temp_range_low_c)
        .bind(input.cool_plate_temp_c)
        .bind(input.cool_plate_temp_initial_c)
        .bind(input.hot_plate_temp_c)
        .bind(input.hot_plate_temp_initial_c)
        .bind(input.textured_plate_temp_c)
        .bind(input.textured_plate_temp_initial_c)
        .bind(input.eng_plate_temp_c)
        .bind(input.eng_plate_temp_initial_c)
        .bind(input.supertack_plate_temp_c)
        .bind(input.supertack_plate_temp_initial_c)
        .bind(input.fan_min_speed_pct)
        .bind(input.fan_max_speed_pct)
        .bind(input.additional_cooling_fan_speed_pct)
        .bind(input.close_fan_first_x_layers)
        .bind(input.complete_print_exhaust_fan_speed_pct)
        .bind(input.during_print_exhaust_fan_speed_pct)
        .bind(input.chamber_temperature_c)
        .bind(input.cooling_perimeter_transition_distance_mm)
        .bind(&input.cooling_slowdown_logic)
        .bind(input.enable_overhang_bridge_fan)
        .bind(input.fan_cooling_layer_time_s)
        .bind(input.first_x_layer_fan_speed_pct)
        .bind(input.full_fan_speed_layer)
        .bind(input.no_slow_down_for_cooling_on_outwalls)
        .bind(input.overhang_fan_speed_pct)
        .bind(&input.overhang_fan_threshold)
        .bind(&input.overhang_threshold_participating_cooling)
        .bind(input.override_process_overhang_speed)
        .bind(input.pre_start_fan_time_s)
        .bind(input.reduce_fan_stop_start_freq)
        .bind(input.slow_down_for_layer_cooling)
        .bind(input.slow_down_layer_time_s)
        .bind(input.slow_down_min_speed_mm_s)
        .bind(input.activate_air_filtration)
        .bind(input.retraction_mm)
        .bind(input.retraction_speed_mm_s)
        .bind(input.deretraction_speed_mm_s)
        .bind(input.retraction_minimum_travel_mm)
        .bind(&input.retract_before_wipe)
        .bind(input.retract_restart_extra_mm)
        .bind(input.retract_when_changing_layer)
        .bind(input.retraction_distances_when_cut_mm)
        .bind(input.retract_length_nc_mm)
        .bind(input.long_retractions_when_cut)
        .bind(input.long_retractions_when_ec)
        .bind(input.retraction_distances_when_ec_mm)
        .bind(input.wipe_enabled)
        .bind(input.wipe_distance_mm)
        .bind(input.z_hop_mm)
        .bind(&input.z_hop_type)
        .bind(input.change_length_mm)
        .bind(input.change_length_nc_mm)
        .bind(input.cooling_before_tower_s)
        .bind(input.minimal_purge_on_wipe_tower_mm3)
        .bind(input.prime_volume_mm3)
        .bind(input.prime_volume_nc_mm3)
        .bind(input.ramming_travel_time_s)
        .bind(input.ramming_travel_time_nc_s)
        .bind(input.ramming_volumetric_speed_mm3_s)
        .bind(input.ramming_volumetric_speed_nc_mm3_s)
        .bind(input.tower_interface_pre_extrusion_dist_mm)
        .bind(input.tower_interface_pre_extrusion_length_mm)
        .bind(input.tower_interface_print_temp_c)
        .bind(input.tower_interface_purge_volume_mm3)
        .bind(input.tower_ironing_area_mm2)
        .bind(input.flush_temp_c)
        .bind(input.flush_volumetric_speed_mm3_s)
        .bind(input.adaptive_volumetric_speed)
        .bind(input.max_volumetric_speed_mm3_s)
        .bind(input.bridge_speed_mm_s)
        .bind(input.enable_overhang_speed)
        .bind(input.overhang_14_speed_mm_s)
        .bind(input.overhang_24_speed_mm_s)
        .bind(input.overhang_34_speed_mm_s)
        .bind(input.overhang_44_speed_mm_s)
        .bind(input.overhang_totally_speed_mm_s)
        .bind(input.circle_compensation_speed_mm_s)
        .bind(input.velocity_adaptation_factor)
        .bind(&input.volumetric_speed_coefficients)
        .bind(input.density_g_cm3)
        .bind(input.diameter_mm)
        .bind(input.diameter_limit_mm)
        .bind(&input.shrink_pct)
        .bind(input.soluble)
        .bind(input.is_support)
        .bind(input.printable)
        .bind(input.adhesiveness_category)
        .bind(input.impact_strength_z)
        .bind(input.cost_per_kg)
        .bind(input.flow_ratio)
        .bind(&input.extruder_variant)
        .bind(&input.slicer_notes)
        .bind(input.required_nozzle_hrc)
        .bind(input.enable_pressure_advance)
        .bind(input.pressure_advance)
        .bind(&input.drying_ams_limitations)
        .bind(input.drying_ams_heat_distortion_temp_c)
        .bind(input.drying_ams_temp_c)
        .bind(input.drying_ams_time_h)
        .bind(input.drying_chamber_bed_temp_c)
        .bind(input.drying_chamber_time_h)
        .bind(input.drying_cooling_temp_c)
        .bind(input.drying_softening_temp_c)
        .bind(input.softening_temp_c)
        .bind(&input.scarf_seam_type)
        .bind(&input.scarf_gap_pct)
        .bind(&input.scarf_height_pct)
        .bind(input.scarf_length_mm)
        .bind(input.hole_coef_1)
        .bind(input.hole_coef_2)
        .bind(input.hole_coef_3)
        .bind(input.hole_limit_max)
        .bind(input.hole_limit_min)
        .bind(input.counter_coef_1)
        .bind(input.counter_coef_2)
        .bind(input.counter_coef_3)
        .bind(input.counter_limit_max)
        .bind(input.counter_limit_min)
        .bind(&input.start_gcode)
        .bind(&input.end_gcode)
        .bind(&input.default_colour_hex)
        .bind(input.source.as_deref().unwrap_or("Manual"))
        .bind(&input.source_slicer)
        .bind(&input.raw_settings_json)
        .bind(&input.source_preset_path)
        .bind(&input.version_name)
        .bind(&input.notes)
        .fetch_one(&pool)
        .await
        .expect("insert failed");

    (StatusCode::OK, Json(ProfileResult { ok: true, errors: None, profile: Some(profile) }))
}

// .NET blocks updating a version already referenced by a Print ("Locked" error). The Prints
// table doesn't exist in this DB yet (not ported), so that check is skipped — same gap pattern
// as Spool's has_profiles/has_prints. Add back once Prints lands.
async fn has_prints(pool: &SqlitePool, profile_id: i64) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM prints WHERE profile_id = ?1")
        .bind(profile_id)
        .fetch_one(pool)
        .await
        .expect("query failed")
        > 0
}

async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(input): Json<ProfileInput>,
) -> (StatusCode, Json<ProfileResult>) {
    if input.name.trim().is_empty() {
        return validation_error("name", "Name is required");
    }
    if has_prints(&pool, id).await {
        return (
            StatusCode::BAD_REQUEST,
            Json(ProfileResult {
                ok: false,
                errors: Some(serde_json::json!({
                    "Locked": "This version has been used in a Print — save as a new version instead."
                })),
                profile: None,
            }),
        );
    }

    let sql = format!(
        "UPDATE print_profiles SET
            spool_id = ?, name = ?, print_speed_mm_s = ?,
            nozzle_temp_c = ?, nozzle_temp_initial_c = ?, nozzle_temp_range_high_c = ?, nozzle_temp_range_low_c = ?,
            cool_plate_temp_c = ?, cool_plate_temp_initial_c = ?, hot_plate_temp_c = ?, hot_plate_temp_initial_c = ?,
            textured_plate_temp_c = ?, textured_plate_temp_initial_c = ?, eng_plate_temp_c = ?, eng_plate_temp_initial_c = ?,
            supertack_plate_temp_c = ?, supertack_plate_temp_initial_c = ?,
            fan_min_speed_pct = ?, fan_max_speed_pct = ?, additional_cooling_fan_speed_pct = ?, close_fan_first_x_layers = ?,
            complete_print_exhaust_fan_speed_pct = ?, during_print_exhaust_fan_speed_pct = ?, chamber_temperature_c = ?,
            cooling_perimeter_transition_distance_mm = ?, cooling_slowdown_logic = ?, enable_overhang_bridge_fan = ?,
            fan_cooling_layer_time_s = ?, first_x_layer_fan_speed_pct = ?, full_fan_speed_layer = ?,
            no_slow_down_for_cooling_on_outwalls = ?, overhang_fan_speed_pct = ?, overhang_fan_threshold = ?,
            overhang_threshold_participating_cooling = ?, override_process_overhang_speed = ?, pre_start_fan_time_s = ?,
            reduce_fan_stop_start_freq = ?, slow_down_for_layer_cooling = ?, slow_down_layer_time_s = ?,
            slow_down_min_speed_mm_s = ?, activate_air_filtration = ?,
            retraction_mm = ?, retraction_speed_mm_s = ?, deretraction_speed_mm_s = ?, retraction_minimum_travel_mm = ?,
            retract_before_wipe = ?, retract_restart_extra_mm = ?, retract_when_changing_layer = ?,
            retraction_distances_when_cut_mm = ?, retract_length_nc_mm = ?, long_retractions_when_cut = ?,
            long_retractions_when_ec = ?, retraction_distances_when_ec_mm = ?,
            wipe_enabled = ?, wipe_distance_mm = ?, z_hop_mm = ?, z_hop_type = ?, change_length_mm = ?, change_length_nc_mm = ?,
            cooling_before_tower_s = ?, minimal_purge_on_wipe_tower_mm3 = ?, prime_volume_mm3 = ?, prime_volume_nc_mm3 = ?,
            ramming_travel_time_s = ?, ramming_travel_time_nc_s = ?, ramming_volumetric_speed_mm3_s = ?,
            ramming_volumetric_speed_nc_mm3_s = ?, tower_interface_pre_extrusion_dist_mm = ?,
            tower_interface_pre_extrusion_length_mm = ?, tower_interface_print_temp_c = ?,
            tower_interface_purge_volume_mm3 = ?, tower_ironing_area_mm2 = ?, flush_temp_c = ?, flush_volumetric_speed_mm3_s = ?,
            adaptive_volumetric_speed = ?, max_volumetric_speed_mm3_s = ?, bridge_speed_mm_s = ?, enable_overhang_speed = ?,
            overhang_14_speed_mm_s = ?, overhang_24_speed_mm_s = ?, overhang_34_speed_mm_s = ?, overhang_44_speed_mm_s = ?,
            overhang_totally_speed_mm_s = ?, circle_compensation_speed_mm_s = ?, velocity_adaptation_factor = ?,
            volumetric_speed_coefficients = ?,
            density_g_cm3 = ?, diameter_mm = ?, diameter_limit_mm = ?, shrink_pct = ?, soluble = ?, is_support = ?, printable = ?,
            adhesiveness_category = ?, impact_strength_z = ?, cost_per_kg = ?, flow_ratio = ?, extruder_variant = ?, slicer_notes = ?,
            required_nozzle_hrc = ?,
            enable_pressure_advance = ?, pressure_advance = ?,
            drying_ams_limitations = ?, drying_ams_heat_distortion_temp_c = ?, drying_ams_temp_c = ?, drying_ams_time_h = ?,
            drying_chamber_bed_temp_c = ?, drying_chamber_time_h = ?, drying_cooling_temp_c = ?, drying_softening_temp_c = ?,
            softening_temp_c = ?,
            scarf_seam_type = ?, scarf_gap_pct = ?, scarf_height_pct = ?, scarf_length_mm = ?,
            hole_coef_1 = ?, hole_coef_2 = ?, hole_coef_3 = ?, hole_limit_max = ?, hole_limit_min = ?,
            counter_coef_1 = ?, counter_coef_2 = ?, counter_coef_3 = ?, counter_limit_max = ?, counter_limit_min = ?,
            start_gcode = ?, end_gcode = ?, default_colour_hex = ?,
            source = COALESCE(?, source), source_slicer = COALESCE(?, source_slicer),
            raw_settings_json = COALESCE(?, raw_settings_json), source_preset_path = COALESCE(?, source_preset_path),
            version_name = ?, notes = ?
        WHERE id = ?
        RETURNING {COLUMNS}"
    );

    let profile = sqlx::query_as::<_, PrintProfile>(&sql)
        .bind(input.spool_id)
        .bind(&input.name)
        .bind(input.print_speed_mm_s)
        .bind(input.nozzle_temp_c)
        .bind(input.nozzle_temp_initial_c)
        .bind(input.nozzle_temp_range_high_c)
        .bind(input.nozzle_temp_range_low_c)
        .bind(input.cool_plate_temp_c)
        .bind(input.cool_plate_temp_initial_c)
        .bind(input.hot_plate_temp_c)
        .bind(input.hot_plate_temp_initial_c)
        .bind(input.textured_plate_temp_c)
        .bind(input.textured_plate_temp_initial_c)
        .bind(input.eng_plate_temp_c)
        .bind(input.eng_plate_temp_initial_c)
        .bind(input.supertack_plate_temp_c)
        .bind(input.supertack_plate_temp_initial_c)
        .bind(input.fan_min_speed_pct)
        .bind(input.fan_max_speed_pct)
        .bind(input.additional_cooling_fan_speed_pct)
        .bind(input.close_fan_first_x_layers)
        .bind(input.complete_print_exhaust_fan_speed_pct)
        .bind(input.during_print_exhaust_fan_speed_pct)
        .bind(input.chamber_temperature_c)
        .bind(input.cooling_perimeter_transition_distance_mm)
        .bind(&input.cooling_slowdown_logic)
        .bind(input.enable_overhang_bridge_fan)
        .bind(input.fan_cooling_layer_time_s)
        .bind(input.first_x_layer_fan_speed_pct)
        .bind(input.full_fan_speed_layer)
        .bind(input.no_slow_down_for_cooling_on_outwalls)
        .bind(input.overhang_fan_speed_pct)
        .bind(&input.overhang_fan_threshold)
        .bind(&input.overhang_threshold_participating_cooling)
        .bind(input.override_process_overhang_speed)
        .bind(input.pre_start_fan_time_s)
        .bind(input.reduce_fan_stop_start_freq)
        .bind(input.slow_down_for_layer_cooling)
        .bind(input.slow_down_layer_time_s)
        .bind(input.slow_down_min_speed_mm_s)
        .bind(input.activate_air_filtration)
        .bind(input.retraction_mm)
        .bind(input.retraction_speed_mm_s)
        .bind(input.deretraction_speed_mm_s)
        .bind(input.retraction_minimum_travel_mm)
        .bind(&input.retract_before_wipe)
        .bind(input.retract_restart_extra_mm)
        .bind(input.retract_when_changing_layer)
        .bind(input.retraction_distances_when_cut_mm)
        .bind(input.retract_length_nc_mm)
        .bind(input.long_retractions_when_cut)
        .bind(input.long_retractions_when_ec)
        .bind(input.retraction_distances_when_ec_mm)
        .bind(input.wipe_enabled)
        .bind(input.wipe_distance_mm)
        .bind(input.z_hop_mm)
        .bind(&input.z_hop_type)
        .bind(input.change_length_mm)
        .bind(input.change_length_nc_mm)
        .bind(input.cooling_before_tower_s)
        .bind(input.minimal_purge_on_wipe_tower_mm3)
        .bind(input.prime_volume_mm3)
        .bind(input.prime_volume_nc_mm3)
        .bind(input.ramming_travel_time_s)
        .bind(input.ramming_travel_time_nc_s)
        .bind(input.ramming_volumetric_speed_mm3_s)
        .bind(input.ramming_volumetric_speed_nc_mm3_s)
        .bind(input.tower_interface_pre_extrusion_dist_mm)
        .bind(input.tower_interface_pre_extrusion_length_mm)
        .bind(input.tower_interface_print_temp_c)
        .bind(input.tower_interface_purge_volume_mm3)
        .bind(input.tower_ironing_area_mm2)
        .bind(input.flush_temp_c)
        .bind(input.flush_volumetric_speed_mm3_s)
        .bind(input.adaptive_volumetric_speed)
        .bind(input.max_volumetric_speed_mm3_s)
        .bind(input.bridge_speed_mm_s)
        .bind(input.enable_overhang_speed)
        .bind(input.overhang_14_speed_mm_s)
        .bind(input.overhang_24_speed_mm_s)
        .bind(input.overhang_34_speed_mm_s)
        .bind(input.overhang_44_speed_mm_s)
        .bind(input.overhang_totally_speed_mm_s)
        .bind(input.circle_compensation_speed_mm_s)
        .bind(input.velocity_adaptation_factor)
        .bind(&input.volumetric_speed_coefficients)
        .bind(input.density_g_cm3)
        .bind(input.diameter_mm)
        .bind(input.diameter_limit_mm)
        .bind(&input.shrink_pct)
        .bind(input.soluble)
        .bind(input.is_support)
        .bind(input.printable)
        .bind(input.adhesiveness_category)
        .bind(input.impact_strength_z)
        .bind(input.cost_per_kg)
        .bind(input.flow_ratio)
        .bind(&input.extruder_variant)
        .bind(&input.slicer_notes)
        .bind(input.required_nozzle_hrc)
        .bind(input.enable_pressure_advance)
        .bind(input.pressure_advance)
        .bind(&input.drying_ams_limitations)
        .bind(input.drying_ams_heat_distortion_temp_c)
        .bind(input.drying_ams_temp_c)
        .bind(input.drying_ams_time_h)
        .bind(input.drying_chamber_bed_temp_c)
        .bind(input.drying_chamber_time_h)
        .bind(input.drying_cooling_temp_c)
        .bind(input.drying_softening_temp_c)
        .bind(input.softening_temp_c)
        .bind(&input.scarf_seam_type)
        .bind(&input.scarf_gap_pct)
        .bind(&input.scarf_height_pct)
        .bind(input.scarf_length_mm)
        .bind(input.hole_coef_1)
        .bind(input.hole_coef_2)
        .bind(input.hole_coef_3)
        .bind(input.hole_limit_max)
        .bind(input.hole_limit_min)
        .bind(input.counter_coef_1)
        .bind(input.counter_coef_2)
        .bind(input.counter_coef_3)
        .bind(input.counter_limit_max)
        .bind(input.counter_limit_min)
        .bind(&input.start_gcode)
        .bind(&input.end_gcode)
        .bind(&input.default_colour_hex)
        .bind(&input.source)
        .bind(&input.source_slicer)
        .bind(&input.raw_settings_json)
        .bind(&input.source_preset_path)
        .bind(&input.version_name)
        .bind(&input.notes)
        .bind(id)
        .fetch_optional(&pool)
        .await
        .expect("update failed");

    match profile {
        Some(profile) => (StatusCode::OK, Json(ProfileResult { ok: true, errors: None, profile: Some(profile) })),
        None => (
            StatusCode::NOT_FOUND,
            Json(ProfileResult { ok: false, errors: Some(serde_json::json!({ "id": "not_found" })), profile: None }),
        ),
    }
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<serde_json::Value>) {
    if has_prints(&pool, id).await {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "ok": false, "error": "has_prints" })));
    }

    let result = sqlx::query("DELETE FROM print_profiles WHERE id = ?1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete failed");

    if result.rows_affected() == 0 {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "not_found" })));
    }

    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}
