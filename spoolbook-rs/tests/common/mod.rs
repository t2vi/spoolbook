// Shared by every test file that exercises a now-gated (POST/PUT/DELETE) route — auth.rs is the
// only place that actually tests the gate itself; every other test file just needs to get past
// it so its own (unrelated) assertions still hold. Sessions are real DB rows now (not a
// recomputable HMAC token), so this needs the test's own pool and is async.
pub async fn auth_cookie_header(pool: &sqlx::SqlitePool) -> String {
    spoolbook_rs::auth::test_only_create_session(pool).await
}

// Prints can't be created over the API any more (only by sending a job, 003/FR-010), so tests
// that need one insert it directly. Takes the old create-request JSON shape
// ({ profileId, spoolId, printerId, input: {...} }) and returns what that request used to:
// (200, { print: { id } }).
#[allow(dead_code)]
pub async fn create_print(pool: &sqlx::SqlitePool, body: Option<serde_json::Value>) -> (axum::http::StatusCode, serde_json::Value) {
    let body = body.unwrap();
    let input = &body["input"];
    let blank_to_null = |v: &serde_json::Value| v.as_str().filter(|s| !s.is_empty()).map(str::to_string);
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO prints (profile_id, spool_id, printer_id, project_id, project_plater_id, started_at, ended_at, status, notes, clean_build_plate)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) RETURNING id",
    )
    .bind(body["profileId"].as_i64())
    .bind(body["spoolId"].as_i64())
    .bind(body["printerId"].as_i64())
    .bind(input["projectId"].as_i64())
    .bind(input["projectPlaterId"].as_str())
    .bind(input["startedAt"].as_str())
    .bind(blank_to_null(&input["endedAt"]))
    .bind(input["status"].as_str())
    .bind(input["notes"].as_str())
    .bind(input["cleanBuildPlate"].as_bool())
    .fetch_one(pool)
    .await
    .unwrap();
    for mode in input["failureModes"].as_array().into_iter().flatten() {
        sqlx::query("INSERT INTO print_failure_modes (print_id, mode) VALUES (?1, ?2)").bind(id).bind(mode.as_str()).execute(pool).await.unwrap();
    }
    (axum::http::StatusCode::OK, serde_json::json!({ "print": { "id": id } }))
}
