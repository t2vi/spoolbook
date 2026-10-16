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

- **FR-001** — `GET /api/prints/{id}` (signed in) returns the Print with its profile version, spool,
  printer (id + name), project, failure modes and every auto-filled field; 404 if missing.
  *Verify: integration*
- **FR-002** — `GET /api/prints` (signed in), newest `started_at` first: `?printerId=` → that
  printer's **5** most recent; `?projectId=` → all of that project's Prints; neither → all.
  *Verify: integration*
- **FR-003** — `GET /api/prints/inventory` (signed in) filters by exact `status`, `printerId`,
  and the spool's filament `brand` and `material` (all combinable); sorts by `sort` =
  `startedAt` (default) | `status` | `printer` with `order` = `asc` | `desc` (default `desc` for
  `startedAt`, `asc` otherwise; ties newest first); pages with `page` (default 1) and `pageSize`
  (default 20; non-positive values fall back to the defaults); returns `{ prints, total, page,
  pageSize, totalPages }` with `totalPages` ≥ 1, each print carrying its spool (with filament),
  printer and ambient temperature. *Verify: integration*
- **FR-004** — `GET /api/prints/{id}/hourly-weather` (signed in) returns that Print's hourly rows
  ordered by hour; empty array if none. *Verify: integration*
- **FR-005** — `GET /api/prints/{id}/readings` (signed in) returns the stored per-minute telemetry
  snapshot (001/FR-030); empty array if none. *Verify: integration*

### Writing

- **FR-010** — A Print is only ever created by sending a job from spoolbook (FR-015): there is
  no way to log a print by hand, because settings recorded from an actual print are trustworthy
  and settings recalled from memory aren't. `POST /api/prints` does not exist (→ 405).
  *Verify: integration*
- **FR-011** — Failure modes are de-duplicated. Any failure mode on a `Success` Print → 400
  `failure_modes_require_failed_or_partial`, nothing saved. *Verify: integration*
- **FR-012** — Annotating (FR-013) rejects with 400 `{ ok: false, error }`, nothing saved: a
  status other than `Success` / `Failed` / `Partial` → `Unknown status.`; a failure mode
  outside the known set → `Unknown failure mode: <mode>.`; a Print still `InProgress` →
  `still_in_progress` (telemetry hasn't finalized it yet). *Verify: integration*
- **FR-013** — `PUT /api/prints/{id}` `{ status, failureModes, notes, actualRoomTempC,
  cleanBuildPlate }` (editor) annotates a finished Print's outcome: corrects the result,
  replaces the failure-mode set wholesale, and sets notes, room temperature and clean-build-plate.
  Nothing else can change — profile version, spool, printer, project + plate, times, and every
  telemetry/auto-filled field stay as recorded. 404 if missing. *Verify: integration*
- **FR-014** — `DELETE /api/prints/{id}` (editor) deletes the Print together with its failure
  modes and hourly weather; an attached telemetry Job is kept but unlinked. 404 if missing.
  *Verify: integration*
- **FR-015** — Sending a job to a printer creates its `InProgress` Print at once (Printers spec,
  send print); telemetry finalizes it (001/FR-021).

### Recommendation

- **FR-020** — `GET /api/prints/recommend-profile?projectId=&currentTempC=` (signed in) picks, among
  that project's Prints, the best by status (`Success` > `Partial` > `Failed` > `InProgress`),
  then by ambient temperature closest to `currentTempC` (Prints without ambient weather, or no
  `currentTempC` given, rank last on this key), then most recent; returns that Print's exact
  profile version, or `null` when the project has no Prints. *Verify: integration*

### Linking telemetry Jobs

- **FR-030** — Telemetry Jobs attach to their Print automatically when it's sent (001/FR-015);
  with no hand-logged prints there is nothing to match by hand, so `GET /api/prints/job-match`
  and `POST /api/prints/{id}/attach-job` do not exist — requests to them fail with a client
  error and match or attach nothing. *Verify: integration*

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

- **FR-050** — `/prints` lists Prints (FR-003) with columns started, filament (with its color
  swatch, 007/FR-050), profile, spool (lot code), printer, status badge and ambient temperature;
  filters for brand, material, printer and status; sortable started / status / printer headers;
  paging with a page-size choice (10 / 20 / 50). Filters, sort and page live in the URL, so a
  view survives refresh, back and sharing. *Verify: e2e*
- **FR-051** — `/prints/[id]` shows the Print's details and failure modes, ambient weather with
  its hourly chart, chamber temperature / AMS humidity and layer-progress charts from the
  telemetry snapshot, the bed photo, and deletes after confirmation. *Verify: e2e*
- **FR-052** — Once a Print is finished, `/prints/[id]` offers "Edit outcome" (editor): result
  (Success / Failed / Partial), failure-mode tags (shown only for Failed / Partial), notes, room
  temperature and clean build plate, saved via FR-013 with errors shown; unsaved edits are
  guarded (002/FR-008). Everything else on the page stays read-only. *Verify: e2e*

## Notes

- **Decisions (2026-10-16 lost-features review):** no hand-logged prints and no manual
  job-match chip (FR-010, FR-030) — the Blazor "Log a print" form was removed in v1.2.0 on
  purpose; only the outcome annotation of sent prints is restored (FR-013, FR-052). The
  standalone Recommend page (removed v0.1.7) stays gone: the recommendation pre-selects the
  profile in the Print modal (FR-020, 004/FR-050).

- Retroactive: behavior read from the code at `3fdca27`.
- **No UI to log or edit a Print (#164):** the frontend has no client for create, update,
  job-match or attach-job. FR-010–FR-013 and FR-030–FR-031 are API-only today; failure-mode
  tagging is unreachable from the UI.
- Recommendation ranks by `ambient_temp_c` (outdoor, auto-fetched), not by the printer's
  chamber temperature — chamber reflects the enclosure's own heat during the print, not the
  conditions you're choosing settings for.
- Bed photo capture at print end is specced with the printer camera (Printers & telemetry).
