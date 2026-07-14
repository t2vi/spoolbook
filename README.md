# spoolbook

Personal FDM 3D-printing notebook. Tracks filaments, spools, print profiles, and individual
prints, so print outcomes can be correlated against settings and ambient conditions over time —
built for one person, dealing with Melbourne's weather swings affecting print quality.

Avalonia desktop app (.NET 10), local SQLite via EF Core. Single user, no auth, no server, no
network dependency beyond two read-only public HTTP calls (weather lookup, filament catalog
sync — see below).

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
- **Print** — a single print job. References a specific Print Profile version rather than
  copying its ~140 settings fields; once any Print references a profile version, that version is
  locked from further in-place edits, so "which settings I used" stays accurate without
  duplicating data. Records status (success/failed/partial), printer, start/end time, notes, and
  ambient conditions — temp/humidity auto-fetched from Open-Meteo for the print window (fixed
  Melbourne coordinates, no location picker — single-user app), or entered manually. Printer
  telemetry (Bambu Lab P2S) is manual start/end entry for now — Bambu has no official local API,
  only unofficial/reverse-engineered LAN MQTT, which isn't worth building against yet; automatic
  logging is deferred.

## Features

- **Filaments** — browse/search the catalog, add/edit entries, sync from the external catalog.
- **Spools** — track individual rolls against a Filament.
- **Profiles** — reusable print settings, generic or spool-specific; import Bambu Studio filament
  presets directly from the JSON files Bambu Studio saves on disk (not `.3mf` project files —
  those just embed the same JSON), including reading whichever fields are present when a preset
  inherits from a system base.
- **Prints** — log a print job with a profile + spool reference, ambient conditions, and outcome.
- **Dashboard** — at-a-glance view across the above.
- **Settings** — Bambu preset directories, filament catalog source (default + user-added
  additional source URLs, merged on every sync), filament DB version/sync status, app version.

## Filament catalog

Filament data is scraped daily by a separate repo,
[`spoolbook-filament-sync`](https://github.com/t2vi/spoolbook-filament-sync), which publishes a
static `data/filament-catalog.json`. This app fetches that file directly and imports new entries
into the live DB — automatically on launch (throttled to once/24h) and on demand via the "Sync
filament catalog" button in Settings → Filaments. No server, no auth — GitHub raw content is the
only host: a static published file already behaves like a minimal read API, so nothing needs to
be deployed or authenticated against. Scraped color names resolve to real hex values (CSS Color
Module Level 4 + a small supplementary filament-marketing list) rather than a flat placeholder.

## Running

```sh
dotnet run --project Spoolbook.Desktop
```

## Testing

```sh
dotnet test
```

## Releasing

See the "Release checklist" in `CLAUDE.md`. Version bump lives in
`Spoolbook.Desktop/Spoolbook.Desktop.csproj`; publishing a GitHub Release triggers
`.github/workflows/release.yml`, which builds and attaches self-contained single-file binaries
for win-x64, osx-x64, osx-arm64, and linux-x64. Release notes go in `docs/releases/`, indexed by
`CHANGELOG.md`. `.github/workflows/tests.yml` runs the test suite on every push/PR.
