use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

// PrintProfile itself stays a real typed struct (used for both the SQL row mapping and the
// outgoing JSON) — the wire-contract question only applies to the *input* shape below.
#[derive(Serialize, sqlx::FromRow, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrintProfile {
    pub id: i64,
    pub filament_id: i64,
    // Not a print_profiles column — populated only by inventory() (the list view needs the
    // brand/material/color to render without a second round trip per row); every other handler
    // leaves this None and serde's default omits it, same as source_preset_path/version_name.
    #[sqlx(skip)]
    pub filament: Option<crate::filaments::Filament>,
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

// The typed, per-column shape the SQL insert/update binds against — fed by
// `profile_field_spec::parse_fields`, not by serde directly (see ProfileInput below for why).
// Same field set as PrintProfile, minus id/is_current_version/created_at (server-assigned),
// filament_id (query param on create, immutable after), and the handful of columns that live at
// the wire's top level instead of inside `fields` (spool_id, name, source, source_slicer,
// raw_settings_json) or aren't reachable from the web UI at all yet (source_preset_path,
// version_name, notes).
#[derive(Default)]
pub struct ParsedProfileFields {
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
}

// The actual wire shape ProfileForm.svelte's save() sends: every profile-setting field value
// nested inside `fields` as a plain string (the tabbed dynamic form built from GET
// /api/profiles/field-spec treats every value as a string regardless of true type — bools as
// "true"/"false", numbers as plain strings — so making the frontend send real typed JSON would
// mean duplicating all ~135 field types into TypeScript for no benefit). `fields` is parsed into
// ParsedProfileFields by profile_field_spec::parse_fields, using GROUPS as the single source of
// truth for each field's name and type — the mirror of that module's field_strings(), which goes
// the other direction for the field-spec response.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    pub fields: std::collections::HashMap<String, String>,
    pub spool_id: Option<i64>,
    pub source: Option<String>,
    pub source_slicer: Option<String>,
    pub raw_settings_json: Option<String>,
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

pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Option<PrintProfile> {
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
    let mut profiles = sqlx::query_as::<_, PrintProfile>(&sql)
        .fetch_all(&pool)
        .await
        .expect("query failed");

    // Per-row lookup rather than a join or an IN-clause batch: matches the "small personal
    // catalog" scale this endpoint already assumes (see the pagination note below).
    for profile in &mut profiles {
        profile.filament = sqlx::query_as::<_, crate::filaments::Filament>(
            "SELECT id, brand, material, variant, color FROM filaments WHERE id = ?1",
        )
        .bind(profile.filament_id)
        .fetch_optional(&pool)
        .await
        .expect("query failed");
    }

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

fn validation_errors(errors: Vec<(&'static str, String)>) -> (StatusCode, Json<ProfileResult>) {
    let map: serde_json::Map<String, serde_json::Value> =
        errors.into_iter().map(|(field, message)| (field.to_string(), serde_json::Value::String(message))).collect();
    (StatusCode::BAD_REQUEST, Json(ProfileResult { ok: false, errors: Some(serde_json::Value::Object(map)), profile: None }))
}

async fn create(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Query(q): Query<FilamentIdQuery>,
    Json(input): Json<ProfileInput>,
) -> (StatusCode, Json<ProfileResult>) {
    if input.name.trim().is_empty() {
        return validation_error("name", "Name is required");
    }
    let parsed = match crate::profile_field_spec::parse_fields(&input.fields) {
        Ok(p) => p,
        Err(errors) => return validation_errors(errors),
    };

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
        .bind(parsed.print_speed_mm_s)
        .bind(parsed.nozzle_temp_c)
        .bind(parsed.nozzle_temp_initial_c)
        .bind(parsed.nozzle_temp_range_high_c)
        .bind(parsed.nozzle_temp_range_low_c)
        .bind(parsed.cool_plate_temp_c)
        .bind(parsed.cool_plate_temp_initial_c)
        .bind(parsed.hot_plate_temp_c)
        .bind(parsed.hot_plate_temp_initial_c)
        .bind(parsed.textured_plate_temp_c)
        .bind(parsed.textured_plate_temp_initial_c)
        .bind(parsed.eng_plate_temp_c)
        .bind(parsed.eng_plate_temp_initial_c)
        .bind(parsed.supertack_plate_temp_c)
        .bind(parsed.supertack_plate_temp_initial_c)
        .bind(parsed.fan_min_speed_pct)
        .bind(parsed.fan_max_speed_pct)
        .bind(parsed.additional_cooling_fan_speed_pct)
        .bind(parsed.close_fan_first_x_layers)
        .bind(parsed.complete_print_exhaust_fan_speed_pct)
        .bind(parsed.during_print_exhaust_fan_speed_pct)
        .bind(parsed.chamber_temperature_c)
        .bind(parsed.cooling_perimeter_transition_distance_mm)
        .bind(&parsed.cooling_slowdown_logic)
        .bind(parsed.enable_overhang_bridge_fan)
        .bind(parsed.fan_cooling_layer_time_s)
        .bind(parsed.first_x_layer_fan_speed_pct)
        .bind(parsed.full_fan_speed_layer)
        .bind(parsed.no_slow_down_for_cooling_on_outwalls)
        .bind(parsed.overhang_fan_speed_pct)
        .bind(&parsed.overhang_fan_threshold)
        .bind(&parsed.overhang_threshold_participating_cooling)
        .bind(parsed.override_process_overhang_speed)
        .bind(parsed.pre_start_fan_time_s)
        .bind(parsed.reduce_fan_stop_start_freq)
        .bind(parsed.slow_down_for_layer_cooling)
        .bind(parsed.slow_down_layer_time_s)
        .bind(parsed.slow_down_min_speed_mm_s)
        .bind(parsed.activate_air_filtration)
        .bind(parsed.retraction_mm)
        .bind(parsed.retraction_speed_mm_s)
        .bind(parsed.deretraction_speed_mm_s)
        .bind(parsed.retraction_minimum_travel_mm)
        .bind(&parsed.retract_before_wipe)
        .bind(parsed.retract_restart_extra_mm)
        .bind(parsed.retract_when_changing_layer)
        .bind(parsed.retraction_distances_when_cut_mm)
        .bind(parsed.retract_length_nc_mm)
        .bind(parsed.long_retractions_when_cut)
        .bind(parsed.long_retractions_when_ec)
        .bind(parsed.retraction_distances_when_ec_mm)
        .bind(parsed.wipe_enabled)
        .bind(parsed.wipe_distance_mm)
        .bind(parsed.z_hop_mm)
        .bind(&parsed.z_hop_type)
        .bind(parsed.change_length_mm)
        .bind(parsed.change_length_nc_mm)
        .bind(parsed.cooling_before_tower_s)
        .bind(parsed.minimal_purge_on_wipe_tower_mm3)
        .bind(parsed.prime_volume_mm3)
        .bind(parsed.prime_volume_nc_mm3)
        .bind(parsed.ramming_travel_time_s)
        .bind(parsed.ramming_travel_time_nc_s)
        .bind(parsed.ramming_volumetric_speed_mm3_s)
        .bind(parsed.ramming_volumetric_speed_nc_mm3_s)
        .bind(parsed.tower_interface_pre_extrusion_dist_mm)
        .bind(parsed.tower_interface_pre_extrusion_length_mm)
        .bind(parsed.tower_interface_print_temp_c)
        .bind(parsed.tower_interface_purge_volume_mm3)
        .bind(parsed.tower_ironing_area_mm2)
        .bind(parsed.flush_temp_c)
        .bind(parsed.flush_volumetric_speed_mm3_s)
        .bind(parsed.adaptive_volumetric_speed)
        .bind(parsed.max_volumetric_speed_mm3_s)
        .bind(parsed.bridge_speed_mm_s)
        .bind(parsed.enable_overhang_speed)
        .bind(parsed.overhang_14_speed_mm_s)
        .bind(parsed.overhang_24_speed_mm_s)
        .bind(parsed.overhang_34_speed_mm_s)
        .bind(parsed.overhang_44_speed_mm_s)
        .bind(parsed.overhang_totally_speed_mm_s)
        .bind(parsed.circle_compensation_speed_mm_s)
        .bind(parsed.velocity_adaptation_factor)
        .bind(&parsed.volumetric_speed_coefficients)
        .bind(parsed.density_g_cm3)
        .bind(parsed.diameter_mm)
        .bind(parsed.diameter_limit_mm)
        .bind(&parsed.shrink_pct)
        .bind(parsed.soluble)
        .bind(parsed.is_support)
        .bind(parsed.printable)
        .bind(parsed.adhesiveness_category)
        .bind(parsed.impact_strength_z)
        .bind(parsed.cost_per_kg)
        .bind(parsed.flow_ratio)
        .bind(&parsed.extruder_variant)
        .bind(&parsed.slicer_notes)
        .bind(parsed.required_nozzle_hrc)
        .bind(parsed.enable_pressure_advance)
        .bind(parsed.pressure_advance)
        .bind(&parsed.drying_ams_limitations)
        .bind(parsed.drying_ams_heat_distortion_temp_c)
        .bind(parsed.drying_ams_temp_c)
        .bind(parsed.drying_ams_time_h)
        .bind(parsed.drying_chamber_bed_temp_c)
        .bind(parsed.drying_chamber_time_h)
        .bind(parsed.drying_cooling_temp_c)
        .bind(parsed.drying_softening_temp_c)
        .bind(parsed.softening_temp_c)
        .bind(&parsed.scarf_seam_type)
        .bind(&parsed.scarf_gap_pct)
        .bind(&parsed.scarf_height_pct)
        .bind(parsed.scarf_length_mm)
        .bind(parsed.hole_coef_1)
        .bind(parsed.hole_coef_2)
        .bind(parsed.hole_coef_3)
        .bind(parsed.hole_limit_max)
        .bind(parsed.hole_limit_min)
        .bind(parsed.counter_coef_1)
        .bind(parsed.counter_coef_2)
        .bind(parsed.counter_coef_3)
        .bind(parsed.counter_limit_max)
        .bind(parsed.counter_limit_min)
        .bind(&parsed.start_gcode)
        .bind(&parsed.end_gcode)
        .bind(&parsed.default_colour_hex)
        .bind(input.source.as_deref().unwrap_or("Manual"))
        .bind(&input.source_slicer)
        .bind(&input.raw_settings_json)
        .bind(None::<String>) // source_preset_path — not reachable from the wire shape yet
        .bind(None::<String>) // version_name — not reachable from the wire shape yet
        .bind(None::<String>) // notes — not reachable from the wire shape yet
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
    _editor: crate::auth::Editor,
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
    let parsed = match crate::profile_field_spec::parse_fields(&input.fields) {
        Ok(p) => p,
        Err(errors) => return validation_errors(errors),
    };

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
        .bind(parsed.print_speed_mm_s)
        .bind(parsed.nozzle_temp_c)
        .bind(parsed.nozzle_temp_initial_c)
        .bind(parsed.nozzle_temp_range_high_c)
        .bind(parsed.nozzle_temp_range_low_c)
        .bind(parsed.cool_plate_temp_c)
        .bind(parsed.cool_plate_temp_initial_c)
        .bind(parsed.hot_plate_temp_c)
        .bind(parsed.hot_plate_temp_initial_c)
        .bind(parsed.textured_plate_temp_c)
        .bind(parsed.textured_plate_temp_initial_c)
        .bind(parsed.eng_plate_temp_c)
        .bind(parsed.eng_plate_temp_initial_c)
        .bind(parsed.supertack_plate_temp_c)
        .bind(parsed.supertack_plate_temp_initial_c)
        .bind(parsed.fan_min_speed_pct)
        .bind(parsed.fan_max_speed_pct)
        .bind(parsed.additional_cooling_fan_speed_pct)
        .bind(parsed.close_fan_first_x_layers)
        .bind(parsed.complete_print_exhaust_fan_speed_pct)
        .bind(parsed.during_print_exhaust_fan_speed_pct)
        .bind(parsed.chamber_temperature_c)
        .bind(parsed.cooling_perimeter_transition_distance_mm)
        .bind(&parsed.cooling_slowdown_logic)
        .bind(parsed.enable_overhang_bridge_fan)
        .bind(parsed.fan_cooling_layer_time_s)
        .bind(parsed.first_x_layer_fan_speed_pct)
        .bind(parsed.full_fan_speed_layer)
        .bind(parsed.no_slow_down_for_cooling_on_outwalls)
        .bind(parsed.overhang_fan_speed_pct)
        .bind(&parsed.overhang_fan_threshold)
        .bind(&parsed.overhang_threshold_participating_cooling)
        .bind(parsed.override_process_overhang_speed)
        .bind(parsed.pre_start_fan_time_s)
        .bind(parsed.reduce_fan_stop_start_freq)
        .bind(parsed.slow_down_for_layer_cooling)
        .bind(parsed.slow_down_layer_time_s)
        .bind(parsed.slow_down_min_speed_mm_s)
        .bind(parsed.activate_air_filtration)
        .bind(parsed.retraction_mm)
        .bind(parsed.retraction_speed_mm_s)
        .bind(parsed.deretraction_speed_mm_s)
        .bind(parsed.retraction_minimum_travel_mm)
        .bind(&parsed.retract_before_wipe)
        .bind(parsed.retract_restart_extra_mm)
        .bind(parsed.retract_when_changing_layer)
        .bind(parsed.retraction_distances_when_cut_mm)
        .bind(parsed.retract_length_nc_mm)
        .bind(parsed.long_retractions_when_cut)
        .bind(parsed.long_retractions_when_ec)
        .bind(parsed.retraction_distances_when_ec_mm)
        .bind(parsed.wipe_enabled)
        .bind(parsed.wipe_distance_mm)
        .bind(parsed.z_hop_mm)
        .bind(&parsed.z_hop_type)
        .bind(parsed.change_length_mm)
        .bind(parsed.change_length_nc_mm)
        .bind(parsed.cooling_before_tower_s)
        .bind(parsed.minimal_purge_on_wipe_tower_mm3)
        .bind(parsed.prime_volume_mm3)
        .bind(parsed.prime_volume_nc_mm3)
        .bind(parsed.ramming_travel_time_s)
        .bind(parsed.ramming_travel_time_nc_s)
        .bind(parsed.ramming_volumetric_speed_mm3_s)
        .bind(parsed.ramming_volumetric_speed_nc_mm3_s)
        .bind(parsed.tower_interface_pre_extrusion_dist_mm)
        .bind(parsed.tower_interface_pre_extrusion_length_mm)
        .bind(parsed.tower_interface_print_temp_c)
        .bind(parsed.tower_interface_purge_volume_mm3)
        .bind(parsed.tower_ironing_area_mm2)
        .bind(parsed.flush_temp_c)
        .bind(parsed.flush_volumetric_speed_mm3_s)
        .bind(parsed.adaptive_volumetric_speed)
        .bind(parsed.max_volumetric_speed_mm3_s)
        .bind(parsed.bridge_speed_mm_s)
        .bind(parsed.enable_overhang_speed)
        .bind(parsed.overhang_14_speed_mm_s)
        .bind(parsed.overhang_24_speed_mm_s)
        .bind(parsed.overhang_34_speed_mm_s)
        .bind(parsed.overhang_44_speed_mm_s)
        .bind(parsed.overhang_totally_speed_mm_s)
        .bind(parsed.circle_compensation_speed_mm_s)
        .bind(parsed.velocity_adaptation_factor)
        .bind(&parsed.volumetric_speed_coefficients)
        .bind(parsed.density_g_cm3)
        .bind(parsed.diameter_mm)
        .bind(parsed.diameter_limit_mm)
        .bind(&parsed.shrink_pct)
        .bind(parsed.soluble)
        .bind(parsed.is_support)
        .bind(parsed.printable)
        .bind(parsed.adhesiveness_category)
        .bind(parsed.impact_strength_z)
        .bind(parsed.cost_per_kg)
        .bind(parsed.flow_ratio)
        .bind(&parsed.extruder_variant)
        .bind(&parsed.slicer_notes)
        .bind(parsed.required_nozzle_hrc)
        .bind(parsed.enable_pressure_advance)
        .bind(parsed.pressure_advance)
        .bind(&parsed.drying_ams_limitations)
        .bind(parsed.drying_ams_heat_distortion_temp_c)
        .bind(parsed.drying_ams_temp_c)
        .bind(parsed.drying_ams_time_h)
        .bind(parsed.drying_chamber_bed_temp_c)
        .bind(parsed.drying_chamber_time_h)
        .bind(parsed.drying_cooling_temp_c)
        .bind(parsed.drying_softening_temp_c)
        .bind(parsed.softening_temp_c)
        .bind(&parsed.scarf_seam_type)
        .bind(&parsed.scarf_gap_pct)
        .bind(&parsed.scarf_height_pct)
        .bind(parsed.scarf_length_mm)
        .bind(parsed.hole_coef_1)
        .bind(parsed.hole_coef_2)
        .bind(parsed.hole_coef_3)
        .bind(parsed.hole_limit_max)
        .bind(parsed.hole_limit_min)
        .bind(parsed.counter_coef_1)
        .bind(parsed.counter_coef_2)
        .bind(parsed.counter_coef_3)
        .bind(parsed.counter_limit_max)
        .bind(parsed.counter_limit_min)
        .bind(&parsed.start_gcode)
        .bind(&parsed.end_gcode)
        .bind(&parsed.default_colour_hex)
        .bind(&input.source)
        .bind(&input.source_slicer)
        .bind(&input.raw_settings_json)
        .bind(None::<String>) // source_preset_path — not reachable from the wire shape yet
        .bind(None::<String>) // version_name — not reachable from the wire shape yet
        .bind(None::<String>) // notes — not reachable from the wire shape yet
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

async fn delete(_editor: crate::auth::Editor, State(pool): State<SqlitePool>, Path(id): Path<i64>) -> (StatusCode, Json<serde_json::Value>) {
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
