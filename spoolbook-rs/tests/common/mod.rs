// Shared by every test file that exercises a now-gated (POST/PUT/DELETE) route — auth.rs is the
// only place that actually tests the gate itself; every other test file just needs to get past
// it so its own (unrelated) assertions still hold. Sessions are real DB rows now (not a
// recomputable HMAC token), so this needs the test's own pool and is async.
pub async fn auth_cookie_header(pool: &sqlx::SqlitePool) -> String {
    spoolbook_rs::auth::test_only_create_session(pool).await
}
