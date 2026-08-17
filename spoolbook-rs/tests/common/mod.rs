// Shared by every test file that exercises a now-gated (POST/PUT/DELETE) route — auth.rs is the
// only place that actually tests the gate itself; every other test file just needs to get past
// it so its own (unrelated) assertions still hold.
pub const TEST_PASSWORD: &str = "spoolbook-rs-test-password";

pub fn ensure_test_password() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| unsafe { std::env::set_var("SPOOLBOOK_ADMIN_PASSWORD", TEST_PASSWORD) });
}

pub fn auth_cookie_header() -> String {
    ensure_test_password();
    format!("spoolbook_editor={}", spoolbook_rs::auth::test_only_cookie_value(TEST_PASSWORD))
}
