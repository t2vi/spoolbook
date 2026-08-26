use crate::bambu_mqtt_payload_parser::ReadingInput;
use serde::Serialize;
use sqlx::SqlitePool;

// Buffers live MQTT telemetry into Jobs/Readings and matches them to the retrospective Print
// form afterward. See docs/adr/0017-printer-telemetry-mqtt-job-print-separation.md — the actual
// MQTT wire client (subscribing to a real printer) is a separate, later slice; this is the pure
// DB logic behind it, exercised directly by tests rather than over a network.
#[derive(Serialize, sqlx::FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PrinterJob {
    pub id: i64,
    pub printer_id: i64,
    pub external_job_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub print_id: Option<i64>,
}

const COLUMNS: &str = "id, printer_id, external_job_id, started_at, ended_at, print_id";

pub async fn record_reading(
    pool: &SqlitePool,
    printer_id: i64,
    external_job_id: &str,
    input: &ReadingInput,
    at: Option<&str>,
) {
    let existing_job_id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM printer_jobs WHERE printer_id = ?1 AND external_job_id = ?2 AND ended_at IS NULL",
    )
    .bind(printer_id)
    .bind(external_job_id)
    .fetch_optional(pool)
    .await
    .expect("query failed");

    let is_new_job = existing_job_id.is_none();
    let job_id = match existing_job_id {
        Some(id) => id,
        None => sqlx::query_scalar::<_, i64>(
            "INSERT INTO printer_jobs (printer_id, external_job_id, started_at)
             VALUES (?1, ?2, COALESCE(?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')))
             RETURNING id",
        )
        .bind(printer_id)
        .bind(external_job_id)
        .bind(at)
        .fetch_one(pool)
        .await
        .expect("insert failed"),
    };

    sqlx::query(
        "INSERT INTO printer_readings (printer_job_id, recorded_at, nozzle_temp_c, bed_temp_c, chamber_temp_c, ams_slot, progress_pct, ams_humidity_pct, layer_num, total_layer_num)
         VALUES (?1, COALESCE(?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(job_id)
    .bind(at)
    .bind(input.nozzle_temp_c)
    .bind(input.bed_temp_c)
    .bind(input.chamber_temp_c)
    .bind(&input.ams_slot)
    .bind(input.progress_pct)
    .bind(input.ams_humidity_pct)
    .bind(input.layer_num)
    .bind(input.total_layer_num)
    .execute(pool)
    .await
    .expect("insert failed");

    // Auto-create-on-send (docs/adr/0017's 2026-08-14 addendum): a brand-new Job attaches
    // straight to the printer's open (InProgress, not yet attached) Print instead of waiting for
    // the retrospective dismissible-chip match — there's no ambiguity, since the Print was
    // created moments before by the same "send" action that produced this Job. Only checked for
    // a *new* job — an already-attached job must never be reassigned to a later open Print.
    if is_new_job {
        let open_print_id = sqlx::query_scalar::<_, i64>(
            "SELECT p.id FROM prints p
             WHERE p.printer_id = ?1 AND p.status = 'InProgress'
               AND p.id NOT IN (SELECT print_id FROM printer_jobs WHERE print_id IS NOT NULL)
             ORDER BY p.started_at DESC
             LIMIT 1",
        )
        .bind(printer_id)
        .fetch_optional(pool)
        .await
        .expect("query failed");

        if let Some(open_print_id) = open_print_id {
            sqlx::query("UPDATE printer_jobs SET print_id = ?1 WHERE id = ?2")
                .bind(open_print_id)
                .bind(job_id)
                .execute(pool)
                .await
                .expect("update failed");
        }
    }
}

#[derive(Serialize, serde::Deserialize, sqlx::FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReadingSnapshot {
    pub recorded_at: String,
    pub chamber_temp_c: Option<f64>,
    pub ams_humidity_pct: Option<i64>,
    pub layer_num: Option<i64>,
    pub total_layer_num: Option<i64>,
    pub progress_pct: Option<i64>,
}

// Collapses a finished Job's raw Readings into a fixed one-minute-bucketed snapshot stored on
// its Print, then deletes the raw rows -- see docs/adr/0032. Last value per bucket, deliberately
// not averaged: averaging would smooth over abrupt real jumps (e.g. a chamber door opening) that
// this feature exists to surface. No-op (leaves telemetry_json untouched) when there are no
// readings for this job -- nothing to collapse.
pub async fn collapse_readings_to_snapshot(pool: &SqlitePool, print_id: i64, printer_job_id: i64) {
    let readings = sqlx::query_as::<_, ReadingSnapshot>(
        "SELECT recorded_at, chamber_temp_c, ams_humidity_pct, layer_num, total_layer_num, progress_pct
         FROM printer_readings WHERE printer_job_id = ?1 ORDER BY recorded_at ASC",
    )
    .bind(printer_job_id)
    .fetch_all(pool)
    .await
    .expect("query failed");

    if readings.is_empty() {
        return;
    }

    let mut buckets: Vec<ReadingSnapshot> = Vec::new();
    for reading in readings {
        // recorded_at is "YYYY-MM-DDTHH:MM:SS.fffZ" -- the first 16 chars are the minute bucket.
        let bucket_key = &reading.recorded_at[..16];
        match buckets.last() {
            Some(last) if &last.recorded_at[..16] == bucket_key => {
                *buckets.last_mut().unwrap() = reading;
            }
            _ => buckets.push(reading),
        }
    }

    let json = serde_json::to_string(&buckets).expect("serialize failed");
    sqlx::query("UPDATE prints SET telemetry_json = ?1 WHERE id = ?2").bind(json).bind(print_id).execute(pool).await.expect("update failed");
    sqlx::query("DELETE FROM printer_readings WHERE printer_job_id = ?1").bind(printer_job_id).execute(pool).await.expect("delete failed");
}

// Converts any already-attached Job whose Print has no telemetry_json snapshot yet -- covers
// Prints that finished before this feature shipped. Idempotent (safe to call every startup, like
// auth::migrate_env_var_admin_if_needed): the WHERE clause only ever matches a Print once.
pub async fn backfill_reading_snapshots(pool: &SqlitePool) {
    let pending = sqlx::query_as::<_, (i64, i64)>(
        "SELECT pj.id, pj.print_id FROM printer_jobs pj JOIN prints p ON p.id = pj.print_id WHERE p.telemetry_json IS NULL",
    )
    .fetch_all(pool)
    .await
    .expect("query failed");

    for (job_id, print_id) in pending {
        collapse_readings_to_snapshot(pool, print_id, job_id).await;
    }
}

// FINISH/FAILED are unambiguous. Everything else (IDLE, or a delta message that omitted
// gcode_state before this one) falls back to Partial rather than guessing — IDLE could mean a
// dropped FINISH right before the idle snapshot, or the printer going idle after a user-
// initiated Stop, and guessing wrong in either direction is worse than a review flag.
fn map_terminal_gcode_state(gcode_state: Option<&str>) -> &'static str {
    match gcode_state {
        Some("FINISH") => "Success",
        Some("FAILED") => "Failed",
        _ => "Partial",
    }
}

// DB-backed fallback for when the caller has no in-memory active_task_id to hand end_job (e.g.
// after a process restart mid-print — see printer_mqtt.rs's handle_message). At most one open
// job per printer under normal operation; ORDER BY started_at DESC is a defensive tie-break, not
// an expected case.
pub async fn find_open_job_external_id(pool: &SqlitePool, printer_id: i64) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT external_job_id FROM printer_jobs WHERE printer_id = ?1 AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
    )
    .bind(printer_id)
    .fetch_optional(pool)
    .await
    .expect("query failed")
}

pub async fn end_job(
    pool: &SqlitePool,
    printer_id: i64,
    external_job_id: &str,
    terminal_gcode_state: Option<&str>,
    at: Option<&str>,
    chamber_temp_c: Option<f64>,
    ams_humidity_pct: Option<i64>,
    camera_registry: &crate::printer_camera::CameraRegistry,
) {
    let job = sqlx::query_as::<_, (i64, Option<i64>)>(
        "SELECT id, print_id FROM printer_jobs WHERE printer_id = ?1 AND external_job_id = ?2 AND ended_at IS NULL",
    )
    .bind(printer_id)
    .bind(external_job_id)
    .fetch_optional(pool)
    .await
    .expect("query failed");

    let Some((job_id, print_id)) = job else { return };

    sqlx::query("UPDATE printer_jobs SET ended_at = COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) WHERE id = ?2")
        .bind(at)
        .bind(job_id)
        .execute(pool)
        .await
        .expect("update failed");

    if let Some(print_id) = print_id {
        // Snapshot the printer's last-known chamber temp / AMS humidity at end-of-print — the
        // only point either is ever known, since neither is client-writable (see prints.rs).
        sqlx::query(
            "UPDATE prints SET ended_at = COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), status = ?2,
             chamber_temp_c = ?3, ams_humidity_pct = ?4
             WHERE id = ?5 AND status = 'InProgress'",
        )
        .bind(at)
        .bind(map_terminal_gcode_state(terminal_gcode_state))
        .bind(chamber_temp_c)
        .bind(ams_humidity_pct)
        .bind(print_id)
        .execute(pool)
        .await
        .expect("update failed");

        crate::weather::fetch_and_store(pool, print_id).await;
        crate::printer_camera::capture_and_store(pool, camera_registry, print_id, printer_id).await;
        collapse_readings_to_snapshot(pool, print_id, job_id).await;
    }
}

// Auto-match candidate for the retrospective Print form: closest unattached Job for this Printer
// by start time, shown as a dismissible chip rather than attached silently.
pub async fn find_match_for_print(pool: &SqlitePool, printer_id: i64, print_started_at: &str) -> Option<PrinterJob> {
    let sql = format!(
        "SELECT {COLUMNS} FROM printer_jobs
         WHERE printer_id = ?1 AND print_id IS NULL
         ORDER BY ABS(julianday(started_at) - julianday(?2))
         LIMIT 1"
    );
    sqlx::query_as::<_, PrinterJob>(&sql)
        .bind(printer_id)
        .bind(print_started_at)
        .fetch_optional(pool)
        .await
        .expect("query failed")
}

pub async fn attach_job_to_print(pool: &SqlitePool, job_id: i64, print_id: i64) {
    sqlx::query("UPDATE printer_jobs SET print_id = ?1 WHERE id = ?2")
        .bind(print_id)
        .bind(job_id)
        .execute(pool)
        .await
        .expect("update failed");
}

// Unattached Jobs (and their Readings, via ON DELETE CASCADE) older than the cutoff are
// discarded — ADR-0017's 7-day retention window. Attached Jobs are kept regardless of age.
pub async fn purge_unattached_jobs_older_than(pool: &SqlitePool, cutoff: &str) {
    sqlx::query("DELETE FROM printer_jobs WHERE print_id IS NULL AND started_at < ?1")
        .bind(cutoff)
        .execute(pool)
        .await
        .expect("delete failed");
}
