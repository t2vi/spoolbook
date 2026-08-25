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
        "INSERT INTO printer_readings (printer_job_id, recorded_at, nozzle_temp_c, bed_temp_c, chamber_temp_c, ams_slot, progress_pct)
         VALUES (?1, COALESCE(?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(job_id)
    .bind(at)
    .bind(input.nozzle_temp_c)
    .bind(input.bed_temp_c)
    .bind(input.chamber_temp_c)
    .bind(&input.ams_slot)
    .bind(input.progress_pct)
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

pub async fn end_job(pool: &SqlitePool, printer_id: i64, external_job_id: &str, terminal_gcode_state: Option<&str>, at: Option<&str>) {
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
        sqlx::query(
            "UPDATE prints SET ended_at = COALESCE(?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), status = ?2
             WHERE id = ?3 AND status = 'InProgress'",
        )
        .bind(at)
        .bind(map_terminal_gcode_state(terminal_gcode_state))
        .bind(print_id)
        .execute(pool)
        .await
        .expect("update failed");

        crate::weather::fetch_and_store(pool, print_id).await;
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
