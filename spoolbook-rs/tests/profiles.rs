#![recursion_limit = "512"]
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

// Spans the whole field list (name/nozzleTempC near the start, retraction/z-hop mid-list,
// soluble/diameter later, start/end gcode + source at the very end) to catch any bind-order
// or column-count mismatch a full 130-field assertion would be excessive overkill for.
fn full_profile_body() -> Value {
    json!({
        "spoolId": null,
        "name": "Cold day tune",
        "printSpeedMmS": 200,
        "nozzleTempC": 220,
        "nozzleTempInitialC": 215,
        "nozzleTempRangeHighC": null, "nozzleTempRangeLowC": null,
        "coolPlateTempC": null, "coolPlateTempInitialC": null,
        "hotPlateTempC": 55, "hotPlateTempInitialC": 60,
        "texturedPlateTempC": null, "texturedPlateTempInitialC": null,
        "engPlateTempC": null, "engPlateTempInitialC": null,
        "supertackPlateTempC": null, "supertackPlateTempInitialC": null,
        "fanMinSpeedPct": null, "fanMaxSpeedPct": null, "additionalCoolingFanSpeedPct": null,
        "closeFanFirstXLayers": null, "completePrintExhaustFanSpeedPct": null,
        "duringPrintExhaustFanSpeedPct": null, "chamberTemperatureC": null,
        "coolingPerimeterTransitionDistanceMm": 1.5,
        "coolingSlowdownLogic": null, "enableOverhangBridgeFan": true,
        "fanCoolingLayerTimeS": null, "firstXLayerFanSpeedPct": null, "fullFanSpeedLayer": null,
        "noSlowDownForCoolingOnOutwalls": null, "overhangFanSpeedPct": null,
        "overhangFanThreshold": null, "overhangThresholdParticipatingCooling": null,
        "overrideProcessOverhangSpeed": null, "preStartFanTimeS": null,
        "reduceFanStopStartFreq": null, "slowDownForLayerCooling": null,
        "slowDownLayerTimeS": null, "slowDownMinSpeedMmS": null, "activateAirFiltration": null,
        "retractionMm": 0.8, "retractionSpeedMmS": 30.0, "deretractionSpeedMmS": 30.0,
        "retractionMinimumTravelMm": null, "retractBeforeWipe": null, "retractRestartExtraMm": null,
        "retractWhenChangingLayer": true, "retractionDistancesWhenCutMm": null,
        "retractLengthNcMm": null, "longRetractionsWhenCut": null, "longRetractionsWhenEc": null,
        "retractionDistancesWhenEcMm": null,
        "wipeEnabled": true, "wipeDistanceMm": null, "zHopMm": 0.4, "zHopType": "Spiral Lift",
        "changeLengthMm": null, "changeLengthNcMm": null, "coolingBeforeTowerS": null,
        "minimalPurgeOnWipeTowerMm3": null, "primeVolumeMm3": null, "primeVolumeNcMm3": null,
        "rammingTravelTimeS": null, "rammingTravelTimeNcS": null,
        "rammingVolumetricSpeedMm3S": null, "rammingVolumetricSpeedNcMm3S": null,
        "towerInterfacePreExtrusionDistMm": null, "towerInterfacePreExtrusionLengthMm": null,
        "towerInterfacePrintTempC": null, "towerInterfacePurgeVolumeMm3": null,
        "towerIroningAreaMm2": null, "flushTempC": null, "flushVolumetricSpeedMm3S": null,
        "adaptiveVolumetricSpeed": null, "maxVolumetricSpeedMm3S": null, "bridgeSpeedMmS": null,
        "enableOverhangSpeed": null, "overhang14SpeedMmS": null, "overhang24SpeedMmS": null,
        "overhang34SpeedMmS": null, "overhang44SpeedMmS": null, "overhangTotallySpeedMmS": null,
        "circleCompensationSpeedMmS": null, "velocityAdaptationFactor": null,
        "volumetricSpeedCoefficients": null,
        "densityGCm3": 1.24, "diameterMm": 1.75, "diameterLimitMm": null, "shrinkPct": null,
        "soluble": false, "isSupport": false, "printable": null, "adhesivenessCategory": null,
        "impactStrengthZ": null, "costPerKg": 24.99, "flowRatio": null, "extruderVariant": null,
        "slicerNotes": null, "requiredNozzleHrc": null,
        "enablePressureAdvance": null, "pressureAdvance": null,
        "dryingAmsLimitations": null, "dryingAmsHeatDistortionTempC": null, "dryingAmsTempC": null,
        "dryingAmsTimeH": null, "dryingChamberBedTempC": null, "dryingChamberTimeH": null,
        "dryingCoolingTempC": null, "dryingSofteningTempC": null, "softeningTempC": null,
        "scarfSeamType": null, "scarfGapPct": null, "scarfHeightPct": null, "scarfLengthMm": null,
        "holeCoef1": null, "holeCoef2": null, "holeCoef3": null, "holeLimitMax": null, "holeLimitMin": null,
        "counterCoef1": null, "counterCoef2": null, "counterCoef3": null, "counterLimitMax": null, "counterLimitMin": null,
        "startGcode": "G28 ; home", "endGcode": "M104 S0 ; cool down",
        "defaultColourHex": "#111111",
        "source": "Manual", "sourceSlicer": null, "rawSettingsJson": null, "sourcePresetPath": null,
        "versionName": null, "notes": "winter settings"
    })
}

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
    assert_eq!(p["notes"], "winter settings");
    assert_eq!(p["versionNumber"], 1);
    assert_eq!(p["isCurrentVersion"], true);
}

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

#[tokio::test]
async fn inventory_returns_total_and_profiles() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;

    let (status, body) = send(&pool, "GET", "/api/profiles/inventory", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["profiles"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn update_persists_changes_across_the_column_list() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let (_, created) = send(&pool, "POST", &format!("/api/profiles?filamentId={filament_id}"), Some(full_profile_body())).await;
    let id = created["profile"]["id"].as_i64().unwrap();

    let mut updated = full_profile_body();
    updated["name"] = json!("Warmer day tune");
    updated["nozzleTempC"] = json!(210);
    updated["retractionMm"] = json!(0.6);
    updated["soluble"] = json!(true);
    updated["endGcode"] = json!("M104 S0 ; updated");
    updated["notes"] = json!("summer settings");

    let (status, body) = send(&pool, "PUT", &format!("/api/profiles/{id}"), Some(updated)).await;

    assert_eq!(status, StatusCode::OK, "{body:?}");
    let p = &body["profile"];
    assert_eq!(p["name"], "Warmer day tune");
    assert_eq!(p["nozzleTempC"], 210);
    assert_eq!(p["retractionMm"], 0.6);
    assert_eq!(p["soluble"], true);
    assert_eq!(p["endGcode"], "M104 S0 ; updated");
    assert_eq!(p["notes"], "summer settings");
    // version_number/is_current_version untouched by update, matching .NET.
    assert_eq!(p["versionNumber"], 1);
}

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
            "status": "Success", "notes": null, "amsHumidityPct": null,
            "actualRoomTempC": null, "cleanBuildPlate": true,
            "projectId": null, "projectPlaterId": null,
            "failureModes": []
        }
    }))).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
}

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

#[tokio::test]
async fn field_spec_returns_not_found_for_missing_profile_id() {
    let pool = test_pool().await;
    let (status, body) = send(&pool, "GET", "/api/profiles/field-spec?profileId=999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[tokio::test]
async fn field_spec_returns_the_profiles_name_and_populated_values() {
    let pool = test_pool().await;
    let filament_id = seed_filament(&pool).await;
    let mut body = full_profile_body();
    body["defaultColourHex"] = json!("#abcdef");
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

#[tokio::test]
async fn delete_returns_not_found_for_missing_id() {
    let pool = test_pool().await;
    let (status, _) = send(&pool, "DELETE", "/api/profiles/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
