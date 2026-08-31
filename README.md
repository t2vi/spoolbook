[![Tests](https://github.com/t2vi/spoolbook/actions/workflows/tests.yml/badge.svg)](https://github.com/t2vi/spoolbook/actions/workflows/tests.yml)
[![Docs Pages builds](https://github.com/t2vi/spoolbook/actions/workflows/pages/pages-build-deployment/badge.svg)](https://github.com/t2vi/spoolbook/actions/workflows/pages/pages-build-deployment)
[![Docker images](https://github.com/t2vi/spoolbook/actions/workflows/docker-images.yml/badge.svg)](https://github.com/t2vi/spoolbook/actions/workflows/docker-images.yml)

# spoolbook

Personal FDM 3D-printing notebook. Tracks filaments, spools, print profiles, and individual
prints, so print outcomes can be correlated against settings and ambient conditions over time —
built for one person, dealing with Melbourne's weather swings affecting print quality.

Self-hosted web app: SvelteKit frontend (`spoolbook-web-svelte`) + a Rust JSON API
(`spoolbook-rs`, axum + sqlx), local SQLite. Single user, shared-secret auth on mutating routes
only (reads stay open), no network dependency beyond a read-only public HTTP call for the
filament catalog sync (see below). The original Avalonia desktop app and the .NET/EF Core backend
it grew into are both retired.

## Domain

- **Filament** — a type: brand + material + variant + color (e.g. "Bambu Lab PLA Matte,
  Charcoal"). Not physical, no quantity. Catalog is seeded from market research and kept fresh
  by an external scraper (see below).
- **Spool** — a specific physical roll of a Filament. Can behave slightly differently from
  another spool of the same Filament (lot variance).
- **Print Profile** — reusable settings for a Filament (optionally spool-specific via a nullable
  `SpoolId`). Stays editable — it's a "current best settings" record, not history: editing one
  in place is the expected workflow for seasonal adjustment (hot day → lower temp, later prints
  just reuse the edited profile).
- **Printer** — a physical printer the user owns, identified by a unique name and optional model.
  Promoted from a free-text string on Print to a first-class entity once more than one printer
  became relevant.
- **Project** — a `.3mf` project file (the actual sliced project sent to the printer), linked by
  path rather than copied in (project files can be tens of MBs). Reusable across multiple Prints
  (e.g. reprinting the same sliced file after a failure), so it's its own entity rather than a
  field on Print. Drift on disk (moved, deleted, overwritten) is detected via a cheap mtime/size
  stat, not a content hash, and surfaced as a non-blocking badge — not a validity gate.
- **Print** — a single print job. References a specific Print Profile version rather than
  copying its ~140 settings fields; once any Print references a profile version, that version is
  locked from further in-place edits, so "which settings I used" stays accurate without
  duplicating data. Records status (success/failed/partial), the Printer used, optionally a
  Project (the `.3mf` that was actually sliced), start/end time, notes, and ambient conditions —
  currently entered manually; `ambient_temp_c`/`ambient_humidity_pct` auto-fetch from Open-Meteo
  existed in the retired .NET backend but hasn't been ported to `spoolbook-rs` yet. Printer
  telemetry (Bambu Lab P2S) is live via LAN MQTT — status, AMS, camera feed.

## Features

- **Filaments** — browse/search the catalog, add/edit entries, sync from the external catalog.
- **Spools** — track individual rolls against a Filament.
- **Profiles** — reusable print settings, generic or spool-specific; import Bambu Studio filament
  presets directly from the JSON files Bambu Studio saves on disk (not `.3mf` project files —
  those just embed the same JSON), including reading whichever fields are present when a preset
  inherits from a system base.
- **Prints** — log a print job with a profile + spool + printer reference, an optional linked
  `.3mf` Project, ambient conditions, and outcome.
- **Dashboard** — at-a-glance view across the above.
- **Settings** — Bambu preset directories, filament catalog source (default + user-added
  additional source URLs, merged on every sync), filament DB version/sync status, app version,
  and the list of owned Printers.

## Filament catalog

Filament data is scraped daily by a separate repo,
[`spoolbook-filament-sync`](https://github.com/t2vi/spoolbook-filament-sync), which publishes a
static `data/filament-catalog.json`. This app fetches that file directly and imports new entries
into the live DB — automatically on launch (throttled to once/24h) and on demand via the "Sync
filament catalog" button in Settings → Filaments. No server, no auth — GitHub raw content is the
only host: a static published file already behaves like a minimal read API, so nothing needs to
be deployed or authenticated against. Scraped color names resolve to real hex values (CSS Color
Module Level 4 + a small supplementary filament-marketing list) rather than a flat placeholder.

## Installing

```sh
curl -fsSL https://raw.githubusercontent.com/t2vi/spoolbook/main/install.sh | bash
```

Pulls the pre-built `spoolbook` + `spoolbook-slicer` (BambuStudio, for re-slicing before print)
images from GHCR into `./spoolbook/`, prompts for an admin password, and starts both containers
via Docker Compose. Requires Docker + the Compose plugin. See `docker-compose.yml` for the
service layout if you'd rather run it by hand.

## Developing

```sh
cd spoolbook-web-svelte && npm run dev   # frontend, port 5173, proxies /api to :5070
cd spoolbook-rs && cargo run             # backend, port 5070, sqlite at spoolbook-rs/dev.db
```

## Testing

```sh
cd spoolbook-rs && cargo test
```

`tests/printer_live.rs` holds hardware-in-the-loop tests (MQTT connect, FTPS upload, send a
print and cancel it in the prep phase, unsliced `.3mf` → slicer-service → print). They're
`#[ignore]`d and need a real printer on the LAN — run them by hand while developing the printer
integration:

```sh
SPOOLBOOK_TEST_PRINTER_IP=… SPOOLBOOK_TEST_PRINTER_ACCESS_CODE=… SPOOLBOOK_TEST_PRINTER_SERIAL=… \
  cargo test --test printer_live -- --ignored --nocapture
```

The print tests also need `SPOOLBOOK_TEST_ALLOW_REAL_PRINT=1` and a `.3mf` path; `examples/stop_print.rs`
is a standalone cancel command if one gets away.

## Releasing

See the "Release checklist" in `CLAUDE.md`. `.github/workflows/docker-images.yml` builds and
pushes the `spoolbook`/`spoolbook-slicer` images to GHCR (gated on a real slicing smoke test
against the latest-stable BambuStudio). Release notes go in `docs/releases/`, indexed by
`CHANGELOG.md`. `.github/workflows/tests.yml` runs the test suite on every push/PR.
