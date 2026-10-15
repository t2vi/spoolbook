# 002 — Settings, dashboard, data export/import, startup & serving

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Settings & data (#153)
**Code:** `spoolbook-rs/src/{settings,dashboard,export_import,lib,main}.rs`; pages `/` and `/settings`
(`spoolbook-web-svelte/src/routes/+page.svelte`, `settings/+page.svelte`)

## Summary

The host-level plumbing: how the process starts and serves the app, the single-row app
settings, the dashboard's summary counts, and moving a whole install's data to another install
via an export zip that is *merged* into the target rather than replacing it.

## Key entities

- **App settings** (`app_settings`, always exactly one row, `id = 1`) — additional filament
  catalog source URLs, latitude/longitude, last catalog sync time, Google OAuth config (spec'd
  with Accounts & auth).
- **Export bundle** — a zip: `manifest.json` (`version` + one array of raw rows per exported
  table) and `files/<name>` for each Project's `.3mf`.

## Functional requirements

### Startup & serving

- **FR-001** — The SQLite file is `SPOOLBOOK_DB_PATH` if set, else `dev.db` in the working
  directory; it is created if missing, and foreign keys are enforced. *Verify: e2e*
- **FR-002** — Before serving, startup runs in order: schema migrations, the env-var admin
  migration (Accounts & auth), the Reading-snapshot backfill (001/FR-031), the per-printer
  telemetry loops (001/FR-001), and — at most once per 24 h — a background filament catalog
  sync (Catalog). *Verify: e2e*
- **FR-003** — The server listens on `0.0.0.0:5070` (reachable from the LAN, not just
  localhost). *Verify: e2e*
- **FR-004** — Non-API paths are served from the frontend's static build at
  `SPOOLBOOK_STATIC_ROOT` (default `../spoolbook-web-svelte/build`); a path matching no file
  returns that build's `index.html` with **200**, so client-side routes deep-link correctly.
  *Verify: e2e*
- **FR-005** — Request bodies up to the project upload limit are accepted on every route (the
  framework's 2 MB default is too small for a real `.3mf`). *Verify: integration*
- **FR-006** — `GET /api/version` (open) returns `{ "version": "<backend crate version>" }`.
  *Verify: integration*
- **FR-007** — Every page shows the version and a "Source" link to the public repository
  (`https://github.com/t2vi/spoolbook`) — in the sidebar when signed in, under the form on the
  sign-in and setup pages otherwise — the offer of source that
  the AGPL-3.0 license requires of anyone running the app for others. *Verify: e2e*
- **FR-008** — Every edit form (filament, color, spool, profile, printer, print outcome,
  settings) tracks unsaved edits: leaving with unsaved edits — an in-app link, back, reload or
  closing the tab — asks for confirmation first; saving or cancelling clears it.
  *Verify: e2e*

### Settings

- **FR-010** — `GET /api/settings` (signed in) returns `additionalFilamentSourceUrls`,
  `lastFilamentSyncAt`, `catalogUrl` (the published catalog URL, or its override — 007/FR-024),
  `latitude`,
  `longitude`. The first read on a fresh DB creates the settings row with all values null.
  *Verify: integration*
- **FR-011** — `POST /api/settings` (editor only) replaces additional source URLs, latitude and
  longitude together — a field omitted from the request is saved as null. Whitespace-only
  source-URL text is saved as null. *Verify: integration*
- **FR-012** — Latitude outside [-90, 90] → 400 `{ ok: false, error: "Latitude must be between
  -90 and 90." }`; longitude outside [-180, 180] → 400 `Longitude must be between -180 and
  180.`; nothing is saved. *Verify: integration*
- **FR-013** — Effects: latitude/longitude drive the ambient weather fetch for ended Prints
  (Prints spec); additional source URLs are fetched alongside the published catalog on sync
  (Catalog spec). Neither has any other effect.
- **FR-014** — The Settings page shows the published catalog URL and last sync time, edits
  additional source URLs (one per line) and location, and confirms a save — or shows the
  server's error when it's refused. *Verify: e2e*

### Dashboard

- **FR-020** — `GET /api/dashboard` (signed in) returns: filament count; filaments grouped by brand
  and by material, each sorted by count descending; spools as **Unopened** (no `opened_at`),
  **Opened** (opened, not emptied), **Empty** (`emptied_at` set); prints by status, always
  listing all four of `Success`, `Failed`, `Partial`, `InProgress` even at zero; last catalog
  sync time; count of *current* profile versions. *Verify: integration*
- **FR-021** — The dashboard page charts spools by status, prints by outcome, and the top 10
  brands and materials, and shows the last catalog sync time when there is one. *Verify: e2e*

### Export

- **FR-030** — `GET /api/export` (editor only) downloads `spoolbook-export.zip` whose
  `manifest.json` holds `version` (this install's version) and every row of `filaments`,
  `filament_colors`, `printers`, `spools`, `print_profiles`, `projects`, `prints`,
  `print_failure_modes`, `print_hourly_weather`, each in id order (so a version is always
  exported after the version it builds on; binary columns base64). Users, sessions, app
  settings and raw telemetry (Jobs/Readings) are never exported. *Verify: integration*
- **FR-031** — Each Project's `.3mf` is bundled as `files/<file name>`. A missing or unreadable
  file is skipped; the export still succeeds. *Verify: integration*

### Import

- **FR-040** — Preview and commit (both editor only) take a multipart `file` field. Missing
  file, not a zip, no `manifest.json`, malformed manifest, or a manifest `version` newer than
  this install → 400 with an error message, nothing changed. *Verify: integration*
- **FR-041** — `POST /api/import/preview` reports, per exported table, `{ total, new }`
  without changing anything. Filaments match existing ones on brand + material + variant +
  color, printers on name, filament colors on name; every other table counts all rows as new.
  *Verify: integration*
- **FR-042** — `POST /api/import/commit` merges, never replaces: matched filaments, printers
  and colors reuse the existing row; every other row is inserted with a fresh id; **every**
  foreign key is rewritten to the target's ids — including `print_profiles.spool_id`,
  `print_profiles.previous_version_profile_id` and `projects.previous_version_project_id`. Returns `{ ok, tables: { <table>: { inserted,
  matched } } }`. *Verify: integration*
- **FR-043** — Commit is all-or-nothing: if any row can't be imported (missing id, a value of
  an unsupported type, a constraint violation such as a reference that can't be resolved), the
  target DB is left exactly as it was and the response is 400 `{ ok: false, error }` naming the
  table — never a partial import or a 500. *Verify: integration*
- **FR-044** — Each imported Project's `.3mf` is written to project storage under its original
  file name unless a file of that name already exists there, and the Project's `file_path`
  points at the target location. Files are written only after the import has succeeded, so a
  failed import (FR-043) leaves none behind. *Verify: integration*
- **FR-045** — The Settings page imports in two steps: choose file → preview counts → confirm
  commit → per-table result. *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **Lost-features review (2026-10-16):** FR-008 restores the desktop app's unsaved-changes
  guard (its edit windows all had one). Dropped on purpose: the desktop's Bambu Studio
  import-path settings (no local preset folder on a server; columns removed in v1.5.0) and its
  About panel (the version is in the sidebar, FR-007; the catalog sync time is on the
  dashboard and in Settings).
- **#157 (fixed):** `save_settings` used to accept any latitude/longitude, so a bad value made
  every later Print's weather fetch fail silently.
- **#158 (fixed):** commit used to skip remapping `print_profiles.spool_id` and
  `projects.previous_version_project_id` — failing the insert, or silently linking the wrong row.
- **#159 (fixed):** commit used to run without a transaction and panic on a bad row, leaving a
  partial import.
- **Re-import (#160):** only filaments, printers and colors dedupe; importing the same bundle
  twice duplicates every Spool, Profile, Project and Print. Intended behavior undecided — no FR
  until it is.
- Exports contain printer access codes (they're columns of `printers`). Editor-only, and the
  file belongs to the editor, but treat the zip as a secret.
- FR-001–FR-004 live in `main.rs`, which no test drives; they need an e2e/smoke layer.
