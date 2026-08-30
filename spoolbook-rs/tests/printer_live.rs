// Hardware-in-the-loop tests. Ignored by default -- they need a real Bambu printer on the LAN,
// powered on and idle. Run them by hand while developing the printer integration:
//
//   SPOOLBOOK_TEST_PRINTER_IP=192.168.1.189 \
//   SPOOLBOOK_TEST_PRINTER_ACCESS_CODE=xxxxxxxx \
//   SPOOLBOOK_TEST_PRINTER_SERIAL=AAAAAAAAAAAAAAA \
//   cargo test --test printer_live -- --ignored --nocapture
//
// Every bug these guard against -- the MQTT max-packet-size flap, the FTPS TLS-session-reuse
// gap, a wire-format the firmware silently rejects -- is invisible to a mock: only the real
// device's own protocol behaviour surfaces it. Not wired into CI (no printer there); this is a
// local dev tool, same as pointing the app at the printer and clicking Print, but repeatable
// and green-or-red.

use spoolbook_rs::printer_camera;
use spoolbook_rs::printer_mqtt::{self, connect_and_subscribe_loop};
use spoolbook_rs::send_print::{build_project_file_payload, compute_gcode_md5, sanitize_for_printer_filename, submission_id};
use sqlx::sqlite::SqlitePoolOptions;
use std::time::{Duration, Instant};

struct PrinterEnv {
    ip: String,
    access_code: String,
    serial: String,
}

/// Returns the printer connection details, or `None` (with a printed skip notice) when the env
/// vars aren't set -- so `cargo test --test printer_live -- --ignored` on a machine with no
/// printer is a visible skip, not a hang or a confusing failure.
fn printer_env() -> Option<PrinterEnv> {
    let ip = std::env::var("SPOOLBOOK_TEST_PRINTER_IP").ok()?;
    let access_code = std::env::var("SPOOLBOOK_TEST_PRINTER_ACCESS_CODE").ok()?;
    let serial = std::env::var("SPOOLBOOK_TEST_PRINTER_SERIAL").ok()?;
    Some(PrinterEnv { ip, access_code, serial })
}

macro_rules! require_printer {
    () => {
        match printer_env() {
            Some(p) => p,
            None => {
                eprintln!(
                    "SKIP: set SPOOLBOOK_TEST_PRINTER_IP / _ACCESS_CODE / _SERIAL to run this against a real printer"
                );
                return;
            }
        }
    };
}

// Seeds printer id 1 so record_reading's job/reading inserts satisfy their FKs -- the loop
// under test writes real telemetry rows the moment the printer reports an active state.
async fn test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("open in-memory db");
    sqlx::migrate!().run(&pool).await.expect("migrations");
    sqlx::query("INSERT INTO printers (id, name, model) VALUES (1, 'live-test', 'P2S')")
        .execute(&pool)
        .await
        .expect("seed printer");
    pool
}

// The whole point: connect the way the app does, confirm the link comes up, confirm a full
// status object lands (pushall -> gcode_state parsed -- the "No live job data yet" symptom), and
// confirm the loop is still connected 35s later (an oversized pushall packet used to error the
// eventloop right after ConnAck, dropping straight back to "Not connected").
#[tokio::test]
#[ignore = "needs a real printer on the LAN"]
async fn live_connection_comes_up_and_stays_up() {
    let p = require_printer!();
    let pool = test_pool().await;
    let store = printer_mqtt::new_store();
    let registry = printer_camera::new_registry();

    let handle = tokio::spawn(connect_and_subscribe_loop(
        1,
        p.ip.clone(),
        p.access_code.clone(),
        p.serial.clone(),
        pool,
        store.clone(),
        registry,
    ));

    // ConnAck within 15s.
    let connected = wait_until(Duration::from_secs(15), || {
        let store = store.clone();
        async move { printer_mqtt::snapshot(&store, 1).await.connected }
    })
    .await;
    assert!(connected, "no MQTT ConnAck within 15s -- check IP / access code / that port 8883 is reachable");

    // A parsed full status (gcode_state) within 30s -- proves pushall was accepted and its
    // response fit the packet-size limit.
    let has_status = wait_until(Duration::from_secs(30), || {
        let store = store.clone();
        async move { printer_mqtt::snapshot(&store, 1).await.gcode_state.is_some() }
    })
    .await;
    assert!(has_status, "connected but no full status in 30s -- pushall rejected or its response overflowed max_packet_size");

    // Still up after a while -- no flap.
    tokio::time::sleep(Duration::from_secs(35)).await;
    assert!(
        printer_mqtt::snapshot(&store, 1).await.connected,
        "connection dropped within ~35s -- the eventloop is erroring (packet size, keepalive, or the printer closed it)"
    );

    handle.abort();
}

// FTPS upload is its own protocol path (implicit TLS on 990, and this printer's vsftpd enforces
// require_ssl_reuse on the data channel -- two rustls FTP stacks failed exactly there). Uploads
// a tiny throwaway file and deletes nothing; a stray 12-byte file in the printer's root is
// harmless and gets overwritten next run.
#[tokio::test]
#[ignore = "needs a real printer on the LAN"]
async fn ftps_upload_roundtrips() {
    let p = require_printer!();

    let local = std::env::temp_dir().join("spoolbook_ftps_probe.txt");
    std::fs::write(&local, b"spoolbook ok").expect("write probe file");

    let result = spoolbook_rs::send_print::upload_via_ftps(
        &p.ip,
        &p.access_code,
        local.to_str().unwrap(),
        "spoolbook_ftps_probe.txt",
    )
    .await;

    std::fs::remove_file(&local).ok();
    assert!(result.is_ok(), "FTPS upload failed: {}", result.unwrap_err());
}

// The full send path, end to end, against the real printer: upload the .3mf over FTPS, publish
// the `project_file` command, watch `gcode_state` transition into the prep phase, then cancel
// before a gram of filament moves. Guards every wire-format bug at once -- a rejected `use_ams`
// (HMS 07FF-8012 -> never reaches PREPARE), a bad md5, the FTPS session-reuse gap, and the
// `stop` command's sequence_id (a non-numeric one is silently ignored and the print runs).
//
// P2S `project_file` order: gcode_state IDLE -> PREPARE (heat bed + nozzle, auto bed-level) ->
// RUNNING (extrusion). Cancelling in PREPARE wastes nothing. From cold, PREPARE can take a few
// minutes -- SPOOLBOOK_TEST_PRINT_PREP_TIMEOUT_SECS overrides the default 240s.
//
// Triple opt-in on purpose: it heats the printer every run, and if `stop` ever regresses the
// printer WILL start printing for real. Needs, all set:
//   SPOOLBOOK_TEST_PRINTER_IP / _ACCESS_CODE / _SERIAL   (as above)
//   SPOOLBOOK_TEST_PRINT_3MF        -- path to a real sliced .3mf with Metadata/<plate>.gcode
//   SPOOLBOOK_TEST_PRINT_PLATE      -- plate gcode name, default "plate_1.gcode"
//   SPOOLBOOK_TEST_ALLOW_REAL_PRINT=1
#[tokio::test]
#[ignore = "sends a real print to the printer (cancelled before extrusion) -- needs SPOOLBOOK_TEST_ALLOW_REAL_PRINT=1"]
async fn send_a_print_then_cancel_before_it_extrudes() {
    let p = require_printer!();
    if !allow_real_print() {
        return;
    }
    let Ok(threemf_path) = std::env::var("SPOOLBOOK_TEST_PRINT_3MF") else {
        eprintln!("SKIP: set SPOOLBOOK_TEST_PRINT_3MF to a real sliced .3mf on disk");
        return;
    };
    let plate = std::env::var("SPOOLBOOK_TEST_PRINT_PLATE").unwrap_or_else(|_| "plate_1.gcode".to_string());

    assert!(
        compute_gcode_md5(&threemf_path, &plate).is_some(),
        "no Metadata/{plate} inside {threemf_path} -- is it a sliced export? use reslice_then_print_then_cancel for an unsliced project"
    );
    print_3mf_then_cancel(&p, std::path::Path::new(&threemf_path), &plate, prep_timeout()).await;
}

// The whole user story in one run: an unsliced project .3mf (a MakerWorld download / saved
// project, no Metadata/*.gcode) -> POST it to the real slicer-service container -> get back a
// sliced .3mf -> send that to the printer -> cancel in prep. Catches a slicer-service output the
// LAN `project_file` protocol can't drive (wrong plate name, missing md5 sidecar, a
// project_settings the P2S rejects), which the pre-sliced test can't see.
//
// Extra to the above, all required:
//   SPOOLBOOK_TEST_PROJECT_3MF   -- path to an UNSLICED project .3mf
//   a slicer-service reachable at RESLICE_SERVICE_URL (default http://localhost:8100), e.g.
//     docker run -d -p 8100:8100 --platform linux/amd64 ghcr.io/t2vi/spoolbook-slicer:latest
#[tokio::test]
#[ignore = "slices then sends a real print (cancelled before extrusion) -- needs SPOOLBOOK_TEST_ALLOW_REAL_PRINT=1 + slicer-service"]
async fn reslice_then_print_then_cancel() {
    let p = require_printer!();
    if !allow_real_print() {
        return;
    }
    let Ok(project_path) = std::env::var("SPOOLBOOK_TEST_PROJECT_3MF") else {
        eprintln!("SKIP: set SPOOLBOOK_TEST_PROJECT_3MF to an unsliced project .3mf on disk");
        return;
    };
    if !slicer_service_up().await {
        eprintln!("SKIP: no slicer-service at {} -- start the spoolbook-slicer container", reslice_base_url());
        return;
    }

    // Precondition: the input really is unsliced (otherwise this isn't testing the slice step).
    assert!(
        compute_gcode_md5(&project_path, "plate_1.gcode").is_none(),
        "{project_path} already contains Metadata/plate_1.gcode -- that's a sliced export, not an unsliced project"
    );

    // Slice it through the real service, exactly as reslicing.rs::slice_via_service does.
    let sliced_bytes = spoolbook_rs::reslicing::slice_via_service(std::path::Path::new(&project_path))
        .await
        .expect("slicer-service call failed");
    let sliced_path = std::env::temp_dir().join(format!("spoolbook-live-sliced-{}.3mf", std::process::id()));
    std::fs::write(&sliced_path, &sliced_bytes).expect("write sliced .3mf");

    // Postcondition: the slice produced a driveable plate gcode.
    let has_gcode = compute_gcode_md5(sliced_path.to_str().unwrap(), "plate_1.gcode").is_some();
    if !has_gcode {
        std::fs::remove_file(&sliced_path).ok();
        panic!("slicer-service returned a .3mf with no Metadata/plate_1.gcode -- check its --slice/--export flags");
    }
    eprintln!("sliced OK ({} bytes) -- sending to printer", sliced_bytes.len());

    print_3mf_then_cancel(&p, &sliced_path, "plate_1.gcode", prep_timeout()).await;
    std::fs::remove_file(&sliced_path).ok();
}

fn allow_real_print() -> bool {
    if std::env::var("SPOOLBOOK_TEST_ALLOW_REAL_PRINT").as_deref() == Ok("1") {
        return true;
    }
    eprintln!("SKIP: set SPOOLBOOK_TEST_ALLOW_REAL_PRINT=1 -- this heats the printer and starts a (cancelled) print");
    false
}

fn prep_timeout() -> Duration {
    Duration::from_secs(
        std::env::var("SPOOLBOOK_TEST_PRINT_PREP_TIMEOUT_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(240),
    )
}

fn reslice_base_url() -> String {
    std::env::var("RESLICE_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8100".to_string())
}

async fn slicer_service_up() -> bool {
    reqwest::Client::new()
        .get(reslice_base_url())
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .is_ok()
}

// Shared tail of both print tests: stand up the telemetry loop, confirm the printer's idle,
// upload + publish `project_file` (use_ams:false), assert it reaches the prep phase, then `stop`
// and assert it actually cancels. Panics with an actionable message at every failure point.
async fn print_3mf_then_cancel(p: &PrinterEnv, threemf_path: &std::path::Path, plate: &str, prep_timeout: Duration) {
    let pool = test_pool().await;
    let store = printer_mqtt::new_store();
    let registry = printer_camera::new_registry();
    let handle = tokio::spawn(connect_and_subscribe_loop(
        1,
        p.ip.clone(),
        p.access_code.clone(),
        p.serial.clone(),
        pool,
        store.clone(),
        registry,
    ));

    // Link up and get one real status before touching anything.
    let ready = wait_until(Duration::from_secs(30), || {
        let store = store.clone();
        async move { printer_mqtt::snapshot(&store, 1).await.gcode_state.is_some() }
    })
    .await;
    assert!(ready, "no telemetry within 30s -- can't safely send a print");

    // Refuse to hijack a print already in progress.
    let state = printer_mqtt::snapshot(&store, 1).await.gcode_state.unwrap_or_default();
    assert!(
        matches!(state.as_str(), "IDLE" | "FINISH" | "FAILED"),
        "printer is busy (gcode_state={state}) -- aborting so this test never interrupts a real job"
    );

    // Upload + publish, same steps as start_print's handler.
    let remote = sanitize_for_printer_filename(threemf_path.file_name().unwrap().to_str().unwrap());
    spoolbook_rs::send_print::upload_via_ftps(&p.ip, &p.access_code, threemf_path.to_str().unwrap(), &remote)
        .await
        .expect("FTPS upload failed");
    let md5 = compute_gcode_md5(threemf_path.to_str().unwrap(), plate)
        .unwrap_or_else(|| panic!("no Metadata/{plate} inside {} -- is it a sliced export?", threemf_path.display()));
    let payload = build_project_file_payload(&remote, &md5, plate, false, 0, true, &submission_id());
    printer_mqtt::publish_raw(&store, 1, &p.serial, payload).await.expect("publish project_file failed");

    // Status must move into the prep/print phase -- this is the assertion that the whole send
    // was accepted (a rejected payload leaves gcode_state at IDLE or flips it to FAILED).
    let started = wait_until(prep_timeout, || {
        let store = store.clone();
        async move {
            matches!(
                printer_mqtt::snapshot(&store, 1).await.gcode_state.as_deref(),
                Some("PREPARE") | Some("RUNNING") | Some("SLICING")
            )
        }
    })
    .await;
    let seen = printer_mqtt::snapshot(&store, 1).await.gcode_state.unwrap_or_default();
    assert!(started, "print never started (gcode_state={seen}) -- payload rejected? check stderr for the HMS code");
    eprintln!("print accepted, gcode_state={seen} -- cancelling now");

    // Cancel. Must actually take.
    printer_mqtt::publish_command(&store, 1, &p.serial, "stop").await.expect("stop command failed to publish");
    let stopped = wait_until(Duration::from_secs(90), || {
        let store = store.clone();
        async move {
            matches!(
                printer_mqtt::snapshot(&store, 1).await.gcode_state.as_deref(),
                Some("IDLE") | Some("FAILED") | Some("FINISH")
            )
        }
    })
    .await;
    let end_state = printer_mqtt::snapshot(&store, 1).await.gcode_state.unwrap_or_default();
    handle.abort();
    assert!(stopped, "STOP did not cancel the print within 90s (gcode_state={end_state}) -- printer is now printing for real, go stop it by hand");
}

/// Polls `check` every 500ms until it returns true or `timeout` elapses.
async fn wait_until<F, Fut>(timeout: Duration, mut check: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if check().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}
