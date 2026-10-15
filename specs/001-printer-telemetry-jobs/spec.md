# 001 — Printer telemetry background jobs

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Printers & telemetry (#151)
**Code:** `spoolbook-rs/src/printer_mqtt.rs`, `spoolbook-rs/src/printer_telemetry.rs`, startup calls in `spoolbook-rs/src/main.rs`

## Summary

Each configured Printer gets a long-lived connection to its own LAN MQTT broker. While a print
runs, its status reports are buffered as **Readings** under a **Job**; when the print ends, the
Job is closed, its attached **Print** is finalized, and the raw Readings are collapsed into a
compact per-minute snapshot stored on that Print. Jobs nobody ever attached to a Print are
discarded after 7 days. All of this runs in the background — no request triggers it.

## Key entities

- **Job** (`printer_jobs`) — one physical print run as seen by the printer: `printer_id`,
  `external_job_id` (the printer's `task_id`), `started_at`, `ended_at` (null while open),
  nullable `print_id` once attached to a Print.
- **Reading** (`printer_readings`) — one buffered status report for a Job. Cascade-deleted with
  its Job.
- **Live status** — in-memory, per Printer: `connected`, `gcode_state`, `errors`, `ams_units`,
  `chamber_temp_c`. Not persisted; lost on restart.
- **Telemetry snapshot** (`prints.telemetry_json`) — the collapsed, per-minute series kept on
  the Print after its Job ends.

## Functional requirements

### Connection lifecycle

- **FR-001** — At startup, a telemetry loop starts for every Printer that has an IP address,
  access code, and serial number all set. A Printer missing any of the three gets no loop.
  *Verify: integration*
- **FR-002** — Creating or updating a Printer (re)starts its loop immediately, without a
  process restart; any existing loop for that Printer is stopped first, so there is never more
  than one loop per Printer. Deleting a Printer stops its loop and clears its live status.
  *Verify: integration*
- **FR-003** — The loop connects to the printer's broker over TLS on port 8883 as user `bblp`
  with the Printer's access code, accepts the printer's self-signed certificate, and subscribes
  read-only to `device/{serial}/report`. Incoming packets up to 1 MB are accepted (a real full
  status report exceeds the client library's 10 KB default). *Verify: live*
- **FR-004** — On every successful connect, the loop publishes one `pushall` request to
  `device/{serial}/request`, so a connection made mid-print shows full status immediately
  instead of waiting for the printer's own slow full-report schedule. *Verify: live*
- **FR-005** — On any connection error the Printer's live status is cleared (reads as
  disconnected), and the loop reconnects after 15 seconds, indefinitely. *Verify: live*
- **FR-006** — With `SPOOLBOOK_MQTT_DEBUG` set (any value), every raw report is written to
  stderr; unset (default), nothing is logged per report. *Verify: live*

### Message handling

- **FR-010** — A payload that doesn't parse (or carries no `gcode_state`) changes nothing: no
  live-status update, no Job or Reading written. *Verify: integration*
- **FR-011** — Every parsed message marks the Printer connected and replaces `gcode_state` and
  `errors` wholesale (a cleared printer error disappears on its own). `ams_units` and
  `chamber_temp_c` are only overwritten when the message carries them — a partial update never
  blanks a last-known value. *Verify: integration*
- **FR-012** — A message in an active print state with a `task_id` records one Reading against
  the open Job for that (Printer, `task_id`), creating the Job (`started_at` = now) if none is
  open. Different `task_id`s are different Jobs. *Verify: integration*
- **FR-013** — A Reading stores nozzle, bed and chamber temperature, AMS slot, progress %, AMS
  humidity %, and current/total layer number. *Verify: integration*
- **FR-014** — A failed Reading write is logged and that one Reading dropped; it never stops the
  Printer's telemetry loop (which also carries pause/resume/stop and print-start commands).
  *Verify: integration*
- **FR-015** — When a Reading creates a *new* Job, that Job attaches to the same Printer's most
  recent `InProgress` Print that no Job is attached to yet. No such Print → the Job stays
  unattached. A Job already created is never re-attached by later Readings. *Verify: integration*

### Ending a Job

- **FR-020** — The first non-active-state message after an active one ends that Job. If the
  in-memory active `task_id` was lost (process restarted mid-print), the Printer's open Job in
  the DB is ended instead. Repeated terminal messages end a Job only once. *Verify: integration*
- **FR-021** — Ending a Job sets its `ended_at`. If it is attached to a Print still
  `InProgress`, that Print gets `ended_at`, a status mapped from the terminal state
  (`FINISH` → `Success`, `FAILED` → `Failed`, anything else → `Partial`), and the Printer's
  last-known chamber temperature and first AMS unit's humidity % (never the coarse 1–5 humidity
  level). A Print not `InProgress` is left untouched; an unattached Job touches no Print.
  *Verify: integration*
- **FR-022** — After finalizing an attached Print, ending its Job also fetches ambient weather
  for the print window, captures a bed photo, and collapses its Readings (FR-030), in that
  order — in the background: ending the Job returns as soon as the Print is finalized, so a
  slow weather API or camera never holds up the printer's telemetry connection.
  *Verify: integration*

### Reading retention

- **FR-030** — Collapsing a Job's Readings buckets them per UTC minute, keeps the **last** value
  in each bucket (no averaging, so abrupt real jumps survive), stores the series on the Print as
  `telemetry_json` (`recordedAt`, `chamberTempC`, `amsHumidityPct`, `layerNum`, `totalLayerNum`,
  `progressPct`), then deletes the raw Readings. A Job with no Readings leaves `telemetry_json`
  untouched. *Verify: integration*
- **FR-031** — At startup, every attached Job whose Print has no `telemetry_json` yet is
  collapsed (covers Prints that finished before collapsing existed). Running it again changes
  nothing. *Verify: integration*
- **FR-032** — Unattached Jobs started more than 7 days ago are deleted with their Readings —
  once at startup, then every 24 hours. Attached Jobs are kept regardless of age.
  *Verify: integration*

### Matching a Job to a Print by hand

- **FR-040** — The match candidate for a Print is the same Printer's unattached Job whose
  `started_at` is closest to the Print's start time; already-attached Jobs are never offered;
  no unattached Jobs → no candidate. *Verify: integration*
- **FR-041** — Attaching a Job to a Print sets the Job's `print_id`. *Verify: integration*

## Notes

- Retroactive: behavior read from the code at `3fdca27`, not designed here. Where code and this
  spec disagree in future, the spec is the intent and the code gets fixed (or the FR amended on
  purpose).
- **FR-032 (#154, fixed):** the running purge loop used to issue its own inline `DELETE`, so the
  purge tests proved a helper production never called. The loop now runs the tested
  `purge_stale_jobs`.
- **FR-022 (#155, fixed):** weather fetch and bed-photo capture used to be awaited inline inside
  the MQTT message handler, stalling that printer's connection. Weather/photo behavior itself is
  specced with Prints (#150).
- **FR-003–FR-006** can only be proven against real hardware (`tests/printer_live.rs`,
  `#[ignore]`d, run by hand); the live layer doesn't count toward coverage.
- Only the first AMS unit's humidity is snapshotted (FR-021); revisit if a multi-AMS setup
  makes that ambiguous.
