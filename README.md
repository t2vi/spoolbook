[![Tests](https://github.com/t2vi/spoolbook/actions/workflows/tests.yml/badge.svg)](https://github.com/t2vi/spoolbook/actions/workflows/tests.yml)
[![Docs Pages builds](https://github.com/t2vi/spoolbook/actions/workflows/pages/pages-build-deployment/badge.svg)](https://github.com/t2vi/spoolbook/actions/workflows/pages/pages-build-deployment)
[![Docker images](https://github.com/t2vi/spoolbook/actions/workflows/docker-images.yml/badge.svg)](https://github.com/t2vi/spoolbook/actions/workflows/docker-images.yml)

# spoolbook

Personal FDM 3D-printing notebook. Tracks filaments, spools, print profiles, and individual
prints, so print outcomes can be correlated against settings and ambient conditions over time —
built for one person, dealing with Melbourne's weather swings affecting print quality.

Self-hosted web app: SvelteKit frontend (`spoolbook-web-svelte`) + a Rust JSON API
(`spoolbook-rs`, axum + sqlx), local SQLite. Single user; nothing is visible without signing
in, no network dependency beyond a read-only public HTTP call for the
filament catalog sync (see below). The original Avalonia desktop app and the .NET/EF Core backend
it grew into are both retired.

## Domain

- **Filament** — a type: brand + material + variant + color (e.g. "Bambu Lab PLA Matte,
  Charcoal"). Not physical, no quantity. Catalog is seeded from market research and kept fresh
  by an external scraper (see below).
- **Spool** — a specific physical roll of a Filament. Can behave slightly differently from
  another spool of the same Filament (lot variance).
- **Print Profile** — reusable settings for a Filament (optionally spool-specific via a nullable
  `SpoolId`). Versioned: its versions are working variants you pick between (e.g. "Winter 2026"
  vs a hot-day tune) — one is selected and used by lists and pickers. A version no print has used
  edits in place; a used one is locked, and "Save as new version" branches a new one from it.
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
  outdoor weather is fetched automatically from Open-Meteo once a print ends; room temperature
  near the printer is entered by hand. Prints are only ever created by sending a job from
  spoolbook (settings come from what was actually printed, not memory); afterwards only the
  outcome (result, failure modes, notes, room temp, clean plate) can be edited. Printer
  telemetry (Bambu Lab P2S) is live via LAN MQTT — status, AMS, camera feed.

## Features

- **Filaments** — browse/search the catalog, add/edit entries, sync from the external catalog.
  **Colors** (Filaments → Colors) manages color names and hex codes; a multi-color filament takes
  several hex codes (`#1A1A1A, #D4AF37`) and its swatch shows each one. Swatches appear next to
  filaments everywhere, and filament pickers are type-to-search.
- **Spools** — track individual rolls against a Filament; filter by brand, material, status and
  purchase date.
- **Profiles** — reusable print settings, generic or for one specific spool; start from a Bambu
  Studio system preset, a sliced `.3mf`, or a preset `.json` Bambu Studio saved (its `inherits`
  chain resolved). Versioned: pick which version is in use, name versions, branch a new one from
  any version; a version used by a print is locked. Duplicate a profile into a new one.
- **Prints** — created automatically when you send a job to a printer (profile + spool + printer
  + `.3mf` Project), then filled in as it runs: outcome, bed photo, outdoor weather, chamber
  temperature, AMS humidity and layer progress. Edit the outcome afterwards; filter the list by
  brand, material, printer and status.
- **Projects** — `.3mf` files, uploaded or fetched from a direct link; a new upload that looks
  like the next version of an existing project offers to link them.
- **Dashboard** — at-a-glance view across the above.
- **Settings** — location (for outdoor weather), filament catalog source (default + user-added
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

## License

[AGPL-3.0-or-later](LICENSE). You can use, modify and redistribute Spoolbook freely; if you run
a modified version for other people — including over a network — you must offer them its
source under the same license. The sidebar's "Source" link points at this repository; a fork
that others use should point it at the fork's own source.
