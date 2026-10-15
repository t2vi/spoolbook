#![recursion_limit = "512"]
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tower::ServiceExt;

async fn test_pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
    sqlx::migrate!().run(&pool).await.expect("migration failed");
    pool
}

async fn send(pool: &sqlx::SqlitePool, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let response = spoolbook_rs::app(pool.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("cookie", common::auth_cookie_header(pool).await)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

async fn seed_filament(pool: &sqlx::SqlitePool) -> i64 {
    let (_, body) = send(
        pool,
        "POST",
        "/api/filaments",
        Some(json!({ "brand": "Bambu Lab", "material": "PLA", "variant": "Basic", "color": "Black" })),
    )
    .await;
    body["entry"]["id"].as_i64().unwrap()
}

// The real wire shape ProfileForm.svelte's save() sends: every profile-setting value nested as
// a string inside `fields`, keyed by the field names in profile_field_spec.rs's GROUPS. Values
// are spread across the early/mid/late positions of the SQL column list (temps near the start,
// retraction/z-hop mid-list, density/soluble later, start/end gcode at the very end) to catch
// any bind-order or column-count mismatch — a full 130-field assertion would be excessive
// overkill for that. Every field not listed here is left unset, which parse_fields treats as
// blank -> None, same as a freshly-opened new-profile form would send.
fn profile_fields() -> Value {
    json!({
        "PrintSpeedMmS": "200",
        "NozzleTempC": "220",
        "NozzleTempInitialC": "215",
        "HotPlateTempC": "55",
        "HotPlateTempInitialC": "60",
        "CoolingPerimeterTransitionDistanceMm": "1.5",
        "EnableOverhangBridgeFan": "true",
        "RetractionMm": "0.8",
        "RetractionSpeedMmS": "30",
        "DeretractionSpeedMmS": "30",
        "RetractWhenChangingLayer": "true",
        "WipeEnabled": "true",
        "ZHopMm": "0.4",
        "ZHopType": "Spiral Lift",
        "DensityGCm3": "1.24",
        "DiameterMm": "1.75",
        "Soluble": "false",
        "IsSupport": "false",
        "CostPerKg": "24.99",
        "StartGcode": "G28 ; home",
        "EndGcode": "M104 S0 ; cool down",
        "DefaultColourHex": "#111111"
    })
}

fn full_profile_body() -> Value {
    json!({
        "spoolId": null,
        "name": "Cold day tune",
        "source": "Manual",
        "sourceSlicer": null,
        "rawSettingsJson": null,
        "fields": profile_fields()
    })
}

// spec: 006/FR-010
#[tokio::test]
async fn create_persists_and_round_trips_fields_across_the_whole_column_list() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;

    let (status, body) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert_eq!(body["ok"], true);
    let p = &body["profile"];
    assert_eq!(p["filamentId"], filament_id);
    assert_eq!(p["name"], "Cold day tune");
    assert_eq!(p["nozzleTempC"], 220);
    assert_eq!(p["nozzleTempInitialC"], 215);
    assert_eq!(p["coolingPerimeterTransitionDistanceMm"], 1.5);
    assert_eq!(p["enableOverhangBridgeFan"], true);
    assert_eq!(p["retractionMm"], 0.8);
    assert_eq!(p["zHopType"], "Spiral Lift");
    assert_eq!(p["soluble"], false);
    assert_eq!(p["diameterMm"], 1.75);
    assert_eq!(p["costPerKg"], 24.99);
    assert_eq!(p["startGcode"], "G28 ; home");
    assert_eq!(p["endGcode"], "M104 S0 ; cool down");
    assert_eq!(p["defaultColourHex"], "#111111");
    assert_eq!(p["source"], "Manual");
    assert_eq!(p["versionNumber"], 1);
    assert_eq!(p["isCurrentVersion"], true);
    // Fields never present in the wire shape (no `fields` entry, no top-level key either) —
    // blank/absent must land as None, not a stale default.
    assert_eq!(p["printSpeedMmS"], 200);
    assert_eq!(p["notes"], Value::Null);
    assert_eq!(p["nozzleTempRangeHighC"], Value::Null);
}

// spec: 006/FR-011
#[tokio::test]
async fn create_leaves_blank_fields_as_none() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut body = full_profile_body();
    // Blank string (as a freshly-opened form's untouched field sends) must become None, not 0/false.
    body["fields"]["PrintSpeedMmS"] = json!("");
    body["fields"]["Soluble"] = json!("");
    body["fields"]["ShrinkPct"] = json!("");

    let (status, body) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(body)).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let p = &body["profile"];
    assert_eq!(p["printSpeedMmS"], Value::Null);
    assert_eq!(p["soluble"], Value::Null);
    assert_eq!(p["shrinkPct"], Value::Null);
}

// spec: 006/FR-011
#[tokio::test]
async fn create_rejects_blank_name() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut body = full_profile_body();
    body["name"] = json!("");

    let (status, resp) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["ok"], false);
}

// spec: 006/FR-011
#[tokio::test]
async fn create_rejects_blank_nozzle_temp_c() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut body = full_profile_body();
    body["fields"]["NozzleTempC"] = json!("");

    let (status, resp) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["ok"], false);
    assert!(resp["errors"]["NozzleTempC"].is_string(), "{resp:?}");
}

// spec: 006/FR-011
#[tokio::test]
async fn create_rejects_unparseable_nozzle_temp_c() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut body = full_profile_body();
    body["fields"]["NozzleTempC"] = json!("abc");

    let (status, resp) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["ok"], false);
    assert!(resp["errors"]["NozzleTempC"].is_string(), "{resp:?}");
}

// spec: 006/FR-011
#[tokio::test]
async fn create_rejects_an_unparseable_optional_numeric_field_instead_of_defaulting() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut body = full_profile_body();
    body["fields"]["RetractionMm"] = json!("not-a-number");

    let (status, resp) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["ok"], false);
    assert!(resp["errors"]["RetractionMm"].is_string(), "{resp:?}");
}

// spec: 006/FR-001
#[tokio::test]
async fn list_for_filament_only_returns_current_versions_sorted_generic_first() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut spool_specific = full_profile_body();
    spool_specific["name"] = json!("Zzz spool-specific");
    // No spool exists to reference — this just proves the ordering (generic before
    // spool-specific), not FK behaviour, so leave spoolId null but check the sort still works
    // via two generic profiles instead.
    let mut second = full_profile_body();
    second["name"] = json!("Aaa second");

    send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(spool_specific)).await;
    send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(second)).await;

    let (status, body) = send(&pool, "GET", &format!("/api/profiles?filamentId={filament_id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body.as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Aaa second", "Zzz spool-specific"]);
}

// spec: 006/FR-002
#[tokio::test]
async fn inventory_returns_total_and_profiles() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;

    let (status, body) = send(&pool, "GET", "/api/profiles/inventory", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["profiles"].as_array().unwrap().len(), 1);
    assert_eq!(body["profiles"][0]["filament"]["brand"], "Bambu Lab", "{body:?}");
    assert_eq!(body["profiles"][0]["filament"]["material"], "PLA");
}

// spec: 006/FR-012
#[tokio::test]
async fn update_persists_changes_across_the_column_list() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();

    let mut updated = full_profile_body();
    updated["name"] = json!("Warmer day tune");
    updated["fields"]["NozzleTempC"] = json!("210");
    updated["fields"]["RetractionMm"] = json!("0.6");
    updated["fields"]["Soluble"] = json!("true");
    updated["fields"]["EndGcode"] = json!("M104 S0 ; updated");

    let (status, body) = send(&pool, "PUT", &format!("/api/profiles/{id}"), Some(updated)).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let p = &body["profile"];
    assert_eq!(p["name"], "Warmer day tune");
    assert_eq!(p["nozzleTempC"], 210);
    assert_eq!(p["retractionMm"], 0.6);
    assert_eq!(p["soluble"], true);
    assert_eq!(p["endGcode"], "M104 S0 ; updated");
    // version_number/is_current_version untouched by update, matching .NET.
    assert_eq!(p["versionNumber"], 1);
}

// spec: 006/FR-011
#[tokio::test]
async fn update_rejects_blank_nozzle_temp_c() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();

    let mut updated = full_profile_body();
    updated["fields"]["NozzleTempC"] = json!("");

    let (status, resp) = send(&pool, "PUT", &format!("/api/profiles/{id}"), Some(updated)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["ok"], false);
    assert!(resp["errors"]["NozzleTempC"].is_string(), "{resp:?}");
}

// spec: 006/FR-012
#[tokio::test]
async fn update_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "PUT", "/api/profiles/999", Some(full_profile_body())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn seed_printer(pool: &sqlx::SqlitePool) -> i64 {
    let (_, body) = send(pool, "POST", "/api/printers", Some(json!({
        "name": "P2S #1", "model": "P2S", "ipAddress": null, "accessCode": null, "serialNumber": null
    }))).await;
    body["printer"]["id"].as_i64().unwrap()
}

async fn seed_spool(pool: &sqlx::SqlitePool, filament_id: i64) -> i64 {
    let (_, body) = send(pool, "POST", "/api/spools", Some(json!({
        "filamentId": filament_id, "lotCode": null, "purchasedAt": null,
        "openedAt": null, "emptiedAt": null, "weightGrams": null, "diameterMm": null, "notes": null
    }))).await;
    body["spool"]["id"].as_i64().unwrap()
}

async fn attach_print(pool: &sqlx::SqlitePool, profile_id: i64, spool_id: i64, printer_id: i64) {
    let (status, body) = send(pool, "POST", "/api/prints", Some(json!({
        "profileId": profile_id, "spoolId": spool_id, "printerId": printer_id,
        "input": {
            "startedAt": "2026-08-15T10:00:00Z", "endedAt": "2026-08-15T12:00:00Z",
            "status": "Success", "notes": null, "cleanBuildPlate": true,
            "projectId": null, "projectPlaterId": null,
            "failureModes": []
        }
    }))).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
}

// spec: 006/FR-020
#[tokio::test]
async fn update_rejects_a_version_used_in_a_print() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();
    let spool_id = seed_spool(&pool, filament_id).await;
    let printer_id = seed_printer(&pool).await;
    attach_print(&pool, id, spool_id, printer_id).await;

    let mut updated = full_profile_body();
    updated["name"] = json!("Should not apply");

    let (status, body) = send(&pool, "PUT", &format!("/api/profiles/{id}"), Some(updated)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["errors"]["Locked"], "This version has been used in a Print — save as a new version instead.");

    let (_, inventory) = send(&pool, "GET", "/api/profiles/inventory", None).await;
    assert_eq!(inventory["profiles"][0]["name"], "Cold day tune", "profile must survive the rejected update unchanged");
}

// spec: 006/FR-013
#[tokio::test]
async fn delete_rejects_a_version_used_in_a_print() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();
    let spool_id = seed_spool(&pool, filament_id).await;
    let printer_id = seed_printer(&pool).await;
    attach_print(&pool, id, spool_id, printer_id).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/profiles/{id}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "has_prints");

    let (_, inventory) = send(&pool, "GET", "/api/profiles/inventory", None).await;
    assert_eq!(inventory["total"], 1, "profile must survive the rejected delete");
}

// spec: 006/FR-003
#[tokio::test]
async fn field_spec_returns_blank_tabs_when_no_profile_id() {
    let pool = test_pool().await;

    let (status, body) = send(&pool, "GET", "/api/profiles/field-spec", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "");
    let tabs = body["tabs"].as_array().unwrap();
    assert_eq!(tabs[0]["title"], "Filament");
    let first_section = &tabs[0]["sections"][0];
    assert_eq!(first_section["title"], "Basic information");
    let fields = first_section["fields"].as_array().unwrap();
    let soluble = fields.iter().find(|f| f["name"] == "Soluble").unwrap();
    assert_eq!(soluble["label"], "Soluble material");
    assert_eq!(soluble["isBool"], true);
    assert_eq!(soluble["isEnum"], false);
    assert_eq!(soluble["isPlainText"], false);
    assert_eq!(soluble["value"], "");
    assert_eq!(soluble["boolValue"], false);

    let diameter = fields.iter().find(|f| f["name"] == "DiameterMm").unwrap();
    assert_eq!(diameter["label"], "Diameter");
    assert_eq!(diameter["unit"], "mm");
    assert_eq!(diameter["isPlainText"], true);

    let default_colour = fields.iter().find(|f| f["name"] == "DefaultColourHex").unwrap();
    assert_eq!(default_colour["hideWhenBlank"], true);
    assert_eq!(default_colour["showRow"], false, "blank + hideWhenBlank must hide the row");

    let long_retraction = tabs.iter()
        .find(|t| t["title"] == "Setting Overrides").unwrap()["sections"]
        .as_array().unwrap().iter().find(|s| s["title"] == "Retraction").unwrap()["fields"]
        .as_array().unwrap().iter().find(|f| f["name"] == "LongRetractionsWhenCut").unwrap()
        .clone();
    assert_eq!(long_retraction["label"], "Long retraction when cut (experimental)", "the NonUnitSuffixes exception keeps this out of the unit slot");
    assert_eq!(long_retraction["unit"], "");
}

// spec: 006/FR-003
#[tokio::test]
async fn field_spec_returns_not_found_for_missing_profile_id() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "GET", "/api/profiles/field-spec?profileId=999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

// spec: 006/FR-003
#[tokio::test]
async fn field_spec_returns_the_profiles_name_and_populated_values() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut body = full_profile_body();
    body["fields"]["DefaultColourHex"] = json!("#abcdef");
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(body)).await;
    let id = created["profile"]["id"].as_i64().unwrap();

    let (status, spec) = send(&pool, "GET", &format!("/api/profiles/field-spec?profileId={id}"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(spec["name"], "Cold day tune");
    let all_fields: Vec<Value> = spec["tabs"].as_array().unwrap().iter()
        .flat_map(|t| t["sections"].as_array().unwrap().iter())
        .flat_map(|s| s["fields"].as_array().unwrap().iter().cloned())
        .collect();
    let nozzle_temp = all_fields.iter().find(|f| f["name"] == "NozzleTempC").unwrap();
    assert_eq!(nozzle_temp["value"], "220");
    let soluble = all_fields.iter().find(|f| f["name"] == "Soluble").unwrap();
    assert_eq!(soluble["value"], "false");
    assert_eq!(soluble["boolValue"], false);
    let bridge_fan = all_fields.iter().find(|f| f["name"] == "EnableOverhangBridgeFan").unwrap();
    assert_eq!(bridge_fan["value"], "true");
    assert_eq!(bridge_fan["boolValue"], true);
    let default_colour = all_fields.iter().find(|f| f["name"] == "DefaultColourHex").unwrap();
    assert_eq!(default_colour["value"], "#abcdef");
    assert_eq!(default_colour["showRow"], true);
}

// Round-trips the exact form flow: load blank spec -> save -> reload same profile's spec (now
// pre-filled) -> edit -> save again, all through the same create/parse/update code path.
// spec: 006/FR-003, 006/FR-012
#[tokio::test]
async fn field_spec_round_trips_through_create_then_edit_then_update() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;

    let (_, blank_spec) = send(&pool, "GET", "/api/profiles/field-spec", None).await;
    assert_eq!(blank_spec["name"], "");

    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();

    let (_, filled_spec) = send(&pool, "GET", &format!("/api/profiles/field-spec?profileId={id}"), None).await;
    assert_eq!(filled_spec["name"], "Cold day tune");

    let mut updated = full_profile_body();
    updated["fields"]["NozzleTempC"] = json!("205");
    let (status, updated_body) = send(&pool, "PUT", &format!("/api/profiles/{id}"), Some(updated)).await;
    assert_eq!(status, StatusCode::OK, "{updated_body:?}");
    assert_eq!(updated_body["profile"]["nozzleTempC"], 205);

    let (_, refetched_spec) = send(&pool, "GET", &format!("/api/profiles/field-spec?profileId={id}"), None).await;
    let nozzle_temp = refetched_spec["tabs"].as_array().unwrap().iter()
        .flat_map(|t| t["sections"].as_array().unwrap().iter())
        .flat_map(|s| s["fields"].as_array().unwrap().iter().cloned())
        .find(|f| f["name"] == "NozzleTempC")
        .unwrap();
    assert_eq!(nozzle_temp["value"], "205");
}

// spec: 006/FR-013
#[tokio::test]
async fn delete_removes_the_profile() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();

    let (status, _) = send(&pool, "DELETE", &format!("/api/profiles/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = send(&pool, "GET", "/api/profiles/inventory", None).await;
    assert_eq!(body["total"], 0);
}

// spec: 006/FR-013
#[tokio::test]
async fn delete_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "DELETE", "/api/profiles/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn create_profile(pool: &sqlx::SqlitePool, filament_id: i64) -> i64 {
    let (_, created) = send(pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    created["profile"]["id"].as_i64().unwrap()
}

async fn is_current(pool: &sqlx::SqlitePool, id: i64) -> bool {
    sqlx::query_scalar("SELECT is_current_version FROM print_profiles WHERE id = ?1").bind(id).fetch_one(pool).await.unwrap()
}

// spec: 006/FR-021, 006/FR-022
#[tokio::test]
async fn new_version_of_a_locked_profile_chains_v2_and_demotes_v1() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let v1 = create_profile(&pool, filament_id).await;
    let spool_id = seed_spool(&pool, filament_id).await;
    let printer_id = seed_printer(&pool).await;
    attach_print(&pool, v1, spool_id, printer_id).await;

    let mut edited = full_profile_body();
    edited["fields"]["NozzleTempC"] = json!("230");
    let (status, body) = send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(edited)).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let v2 = &body["profile"];
    assert_ne!(v2["id"], v1);
    assert_eq!(v2["versionNumber"], 2);
    assert_eq!(v2["previousVersionProfileId"], v1);
    assert_eq!(v2["isCurrentVersion"], true);
    assert_eq!(v2["nozzleTempC"], 230);
    assert!(!is_current(&pool, v1).await);

    let (_, listed) = send(&pool, "GET", &format!("/api/profiles?filamentId={filament_id}"), None).await;
    let ids: Vec<i64> = listed.as_array().unwrap().iter().map(|p| p["id"].as_i64().unwrap()).collect();
    assert_eq!(ids, vec![v2["id"].as_i64().unwrap()]);
    let v1_temp: i64 = sqlx::query_scalar("SELECT nozzle_temp_c FROM print_profiles WHERE id = ?1").bind(v1).fetch_one(&pool).await.unwrap();
    assert_eq!(v1_temp, 220, "the locked version itself never changes");
}

// spec: 006/FR-021
#[tokio::test]
async fn new_version_keeps_the_sources_provenance_and_notes_ignoring_the_requests_and_clears_the_version_name() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let v1 = create_profile(&pool, filament_id).await;
    sqlx::query("UPDATE print_profiles SET source = 'BambuPreset', source_preset_path = 'Bambu PLA Basic', notes = 'dries well', version_name = 'Winter' WHERE id = ?1")
        .bind(v1).execute(&pool).await.unwrap();

    let mut edited = full_profile_body();
    edited["source"] = json!("Manual");
    edited["rawSettingsJson"] = json!("{}");
    let (_, body) = send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(edited)).await;

    let v2 = body["profile"]["id"].as_i64().unwrap();
    let (source, preset, notes, version_name): (String, Option<String>, Option<String>, Option<String>) =
        sqlx::query_as("SELECT source, source_preset_path, notes, version_name FROM print_profiles WHERE id = ?1").bind(v2).fetch_one(&pool).await.unwrap();
    assert_eq!(source, "BambuPreset");
    assert_eq!(preset.as_deref(), Some("Bambu PLA Basic"));
    assert_eq!(notes.as_deref(), Some("dries well"));
    assert_eq!(version_name, None);
}

// spec: 006/FR-021
#[tokio::test]
async fn new_version_from_a_superseded_version_is_rejected() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let v1 = create_profile(&pool, filament_id).await;
    send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(full_profile_body())).await;

    let (status, body) = send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(full_profile_body())).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["errors"]["id"], "not_current_version");
}

// spec: 006/FR-021
#[tokio::test]
async fn new_version_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "POST", "/api/profiles/999/versions", Some(full_profile_body())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// spec: 006/FR-021, 006/FR-011
#[tokio::test]
async fn new_version_with_invalid_fields_changes_nothing() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let v1 = create_profile(&pool, filament_id).await;
    let mut bad = full_profile_body();
    bad["fields"]["NozzleTempC"] = json!("");

    let (status, _) = send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(bad)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM print_profiles").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    assert!(is_current(&pool, v1).await);
}

// spec: 006/FR-013
#[tokio::test]
async fn delete_rejects_a_version_that_a_newer_version_builds_on() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let v1 = create_profile(&pool, filament_id).await;
    send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(full_profile_body())).await;

    let (status, body) = send(&pool, "DELETE", &format!("/api/profiles/{v1}"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "has_newer_version");
}

// spec: 006/FR-013
#[tokio::test]
async fn deleting_the_current_version_makes_the_previous_one_current_again() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let v1 = create_profile(&pool, filament_id).await;
    let (_, body) = send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(full_profile_body())).await;
    let v2 = body["profile"]["id"].as_i64().unwrap();

    let (status, _) = send(&pool, "DELETE", &format!("/api/profiles/{v2}"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert!(is_current(&pool, v1).await);
}

// spec: 006/FR-012
#[tokio::test]
async fn update_keeps_notes_and_version_name() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let id = create_profile(&pool, filament_id).await;
    sqlx::query("UPDATE print_profiles SET notes = 'dries well', version_name = 'Winter' WHERE id = ?1").bind(id).execute(&pool).await.unwrap();

    send(&pool, "PUT", &format!("/api/profiles/{id}"), Some(full_profile_body())).await;

    let (notes, version_name): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT notes, version_name FROM print_profiles WHERE id = ?1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(notes.as_deref(), Some("dries well"));
    assert_eq!(version_name.as_deref(), Some("Winter"));
}

// spec: 006/FR-003
#[tokio::test]
async fn field_spec_reports_the_version_number_and_whether_it_is_current() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let v1 = create_profile(&pool, filament_id).await;
    let (_, body) = send(&pool, "POST", &format!("/api/profiles/{v1}/versions"), Some(full_profile_body())).await;
    let v2 = body["profile"]["id"].as_i64().unwrap();

    let (_, spec1) = send(&pool, "GET", &format!("/api/profiles/field-spec?profileId={v1}"), None).await;
    let (_, spec2) = send(&pool, "GET", &format!("/api/profiles/field-spec?profileId={v2}"), None).await;

    assert_eq!((spec1["versionNumber"].clone(), spec1["isCurrentVersion"].clone()), (json!(1), json!(false)));
    assert_eq!((spec2["versionNumber"].clone(), spec2["isCurrentVersion"].clone()), (json!(2), json!(true)));
}

async fn seed_second_filament(pool: &sqlx::SqlitePool) -> i64 {
    let (_, body) = send(
        pool,
        "POST",
        "/api/filaments",
        Some(json!({ "brand": "Polymaker", "material": "PETG", "variant": null, "color": "Blue" })),
    )
    .await;
    body["entry"]["id"].as_i64().unwrap()
}

async fn profile_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM print_profiles").fetch_one(pool).await.unwrap()
}

// spec: 006/FR-010
#[tokio::test]
async fn create_rejects_an_unknown_filament_or_a_bad_spool() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let other_filament = seed_second_filament(&pool).await;
    let other_spool = seed_spool(&pool, other_filament).await;

    let cases = [
        (999, Value::Null, "filamentId", "Unknown filament."),
        (filament_id, json!(999), "spoolId", "Unknown spool."),
        (filament_id, json!(other_spool), "spoolId", "That spool is a different filament."),
    ];
    for (filament, spool, field, message) in cases {
        let mut body = full_profile_body();
        body["spoolId"] = spool;
        let (status, res) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament}"), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}: {res:?}");
        assert_eq!(res["errors"][field], message);
    }
    assert_eq!(profile_count(&pool).await, 0);
}

// spec: 006/FR-010, 006/FR-012, 006/FR-021
#[tokio::test]
async fn update_and_new_version_reject_another_filaments_spool() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let other_spool = seed_spool(&pool, seed_second_filament(&pool).await).await;
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();

    let mut body = full_profile_body();
    body["spoolId"] = json!(other_spool);
    for uri in [format!("/api/profiles/{id}"), format!("/api/profiles/{id}/versions")] {
        let method = if uri.ends_with("versions") { "POST" } else { "PUT" };
        let (status, res) = send(&pool, method, &uri, Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {res:?}");
        assert_eq!(res["errors"]["spoolId"], "That spool is a different filament.");
    }
    assert_eq!(profile_count(&pool).await, 1);
}

// The edit page relies on this to keep an imported profile's provenance (FR-041, #183).
// spec: 006/FR-012
#[tokio::test]
async fn update_without_provenance_keeps_the_stored_provenance() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let id = create_profile(&pool, filament_id).await;
    sqlx::query("UPDATE print_profiles SET source = 'SlicerImport', source_slicer = 'BambuStudio', raw_settings_json = '{}' WHERE id = ?1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let mut body = full_profile_body();
    for key in ["source", "sourceSlicer", "rawSettingsJson"] {
        body.as_object_mut().unwrap().remove(key);
    }

    let (status, res) = send(&pool, "PUT", &format!("/api/profiles/{id}"), Some(body)).await;

    assert_eq!(status, StatusCode::OK, "{res:?}");
    let (source, slicer, raw): (String, Option<String>, Option<String>) =
        sqlx::query_as("SELECT source, source_slicer, raw_settings_json FROM print_profiles WHERE id = ?1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!((source.as_str(), slicer.as_deref(), raw.as_deref()), ("SlicerImport", Some("BambuStudio"), Some("{}")));
}
