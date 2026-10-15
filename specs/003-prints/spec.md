# 003 — Prints

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Prints (#150)
**Code:** `spoolbook-rs/src/prints.rs`, `spoolbook-rs/src/weather.rs`; pages `/prints`, `/prints/[id]`

## Summary

A **Print** is the retrospective record of one print: which profile version, spool, printer
and (optionally) project plate produced it, when, and how it turned out. Most Prints are
created automatically when spoolbook sends a job to a printer and finalized by telemetry when it
ends (001); this spec covers reading them, the editing API, the best-profile recommendation,
and the ambient weather attached once a Print ends.

## Key entities

- **Print** (`prints`) — `profile_id`, `spool_id`, `printer_id`, nullable `project_id` +
  `project_plater_id`, `started_at`, `ended_at`, `status` (`Success` | `Failed` | `Partial` |
  `InProgress`), `notes`, `clean_build_plate`. Auto-filled, never client-writable:
  `chamber_temp_c`, `ams_humidity_pct` (001/FR-021), `ambient_temp_c`,
  `ambient_humidity_pct`, `ambient_source`, `bed_photo_base64`, `telemetry_json`.
- **Failure mode** (`print_failure_modes`) — zero or more tags per Print: `Stringing`,
  `LayerAdhesion`, `Warping`, `UnderExtrusion`, `OverExtrusion`, `LayerShift`, `Clog`, `Other`.
- **Hourly weather** (`print_hourly_weather`) — one row per (Print, hour).

## Functional requirements

### Reading

- **FR-001** — `GET /api/prints/{id}` (open) returns the Print with its profile version, spool,
  printer (id + name), project, failure modes and every auto-filled field; 404 if missing.
  *Verify: integration*
- **FR-002** — `GET /api/prints` (open), newest `started_at` first: `?printerId=` → that
  printer's **5** most recent; `?projectId=` → all of that project's Prints; neither → all.
  *Verify: integration*
- **FR-003** — `GET /api/prints/inventory` (open) filters by exact `status` and `printerId`,
  pages with `page` (default 1) and `pageSize` (default 20; non-positive values fall back to the
  defaults), newest first, and returns `{ prints, total, page, pageSize, totalPages }` with
  `totalPages` ≥ 1. *Verify: integration*
- **FR-004** — `GET /api/prints/{id}/hourly-weather` (open) returns that Print's hourly rows
  ordered by hour; empty array if none. *Verify: integration*
- **FR-005** — `GET /api/prints/{id}/readings` (open) returns the stored per-minute telemetry
  snapshot (001/FR-030); empty array if none. *Verify: integration*

### Writing

- **FR-010** — `POST /api/prints` (editor) creates a Print from profile, spool, printer,
  start, end, status, notes, clean-build-plate, optional project + plate, and failure modes;
  returns `{ ok, print }` with the full Print. Auto-filled fields can't be set this way.
  *Verify: integration*
- **FR-011** — Failure modes are de-duplicated. Any failure mode on a `Success` Print → 400
  `failure_modes_require_failed_or_partial`, nothing saved (create and update).
  *Verify: integration*
- **FR-012** — Create/update reject with 400 `{ ok: false, error }` (nothing saved): a status
  outside the four values → `Unknown status.`; a failure mode outside the known set →
  `Unknown failure mode: <mode>.`; a profile/spool/printer/project id that doesn't exist →
  `Unknown print profile.` / `Unknown spool.` / `Unknown printer.` / `Unknown project.`; an
  `endedAt` before `startedAt` → `Ended before it started.` (a blank `endedAt` is fine).
  *Verify: integration*
- **FR-013** — `PUT /api/prints/{id}` (editor) updates printer, project + plate, times,
  status, notes, clean-build-plate, and replaces the failure-mode set wholesale. Profile and
  spool are fixed at creation. 404 if missing. *Verify: integration*
- **FR-014** — `DELETE /api/prints/{id}` (editor) deletes the Print together with its failure
  modes and hourly weather; an attached telemetry Job is kept but unlinked. 404 if missing.
  *Verify: integration*
- **FR-015** — Sending a job to a printer creates its `InProgress` Print at once (Printers spec,
  send print); telemetry finalizes it (001/FR-021).

### Recommendation

- **FR-020** — `GET /api/prints/recommend-profile?projectId=&currentTempC=` (open) picks, among
  that project's Prints, the best by status (`Success` > `Partial` > `Failed` > `InProgress`),
  then by ambient temperature closest to `currentTempC` (Prints without ambient weather, or no
  `currentTempC` given, rank last on this key), then most recent; returns that Print's exact
  profile version, or `null` when the project has no Prints. *Verify: integration*

### Linking telemetry Jobs

- **FR-030** — `GET /api/prints/job-match?printerId=&startedAt=` (open) returns the 001/FR-040
  candidate Job or `null`. *Verify: integration*
- **FR-031** — `POST /api/prints/{id}/attach-job` `{ jobId }` (editor) attaches the Job
  (001/FR-041). Missing Print or Job → 404 `not_found`; a Job already attached to a different
  Print → 409 `already_attached` (attaching it again to the same Print is fine).
  *Verify: integration*

### Ambient weather

- **FR-040** — When an attached Job ends (001/FR-022) and a location is set (002/FR-013), the
  Open-Meteo historical archive's hourly temperature and relative humidity are fetched for
  every hour overlapping `[started_at, ended_at]` — including the hour containing
  `started_at`. Their averages go to `ambient_temp_c` / `ambient_humidity_pct` with
  `ambient_source = 'open-meteo'`, and each hour is upserted into hourly weather.
  *Verify: integration*
- **FR-041** — No location, no `ended_at`, an unreachable/failed fetch, or no usable hours →
  ambient fields and hourly weather stay empty; no retry, and ending the Job is never failed by
  it. *Verify: integration*
- **FR-042** — The weather request gives up after 10 s, so a hung API can't leave the fetch
  pending forever. *Verify: integration*
- **FR-043** — `ARCHIVE_URL` overrides the archive endpoint (used by tests).
  *Verify: integration*

### Pages

- **FR-050** — `/prints` lists Prints newest first, 20 per page, with a status filter, an
  optional printer filter from `?printerId=`, and a status badge per row. *Verify: e2e*
- **FR-051** — `/prints/[id]` shows the Print's details and failure modes, ambient weather with
  its hourly chart, chamber temperature / AMS humidity and layer-progress charts from the
  telemetry snapshot, the bed photo, and deletes after confirmation. *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **No UI to log or edit a Print (#164):** the frontend has no client for create, update,
  job-match or attach-job. FR-010–FR-013 and FR-030–FR-031 are API-only today; failure-mode
  tagging is unreachable from the UI.
- Recommendation ranks by `ambient_temp_c` (outdoor, auto-fetched), not by the printer's
  chamber temperature — chamber reflects the enclosure's own heat during the print, not the
  conditions you're choosing settings for.
- Bed photo capture at print end is specced with the printer camera (Printers & telemetry).
