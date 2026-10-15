# 007 — Catalog: filaments, colors, catalog sync, spools

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Catalog (#148)
**Code:** `spoolbook-rs/src/{filaments,colors,filament_catalog_sync,spools}.rs`; pages `/filaments`
(+ `new`, `edit/[id]`), `/spools` (+ `new`, `edit/[id]`)

## Summary

The **filament** catalog (brand / material / variant / color — what you *could* buy) is filled
by hand and by syncing a published catalog feed. A **spool** is one physical roll of a catalog
filament. Colors are discovered from filaments as they're added.

## Key entities

- **Filament** (`filaments`) — `brand`, `material`, optional `variant`, `color`; unique on all
  four (a null variant matches only another null variant).
- **Filament color** (`filament_colors`) — unique `name`, `hex`.
- **Spool** (`spools`) — `filament_id`, `lot_code`, `purchased_at`, `opened_at`, `emptied_at`,
  `weight_grams`, `diameter_mm`, `notes`.

## Functional requirements

### Filaments

- **FR-001** — `GET /api/filaments` (open) filters by exact `brand` and `material`, ordered by
  brand then material, paged (`page` default 1, `pageSize` default 20; non-positive → default),
  returning `{ entries, total, page, pageSize, totalPages ≥ 1 }`. `GET /api/filaments/all`
  (open) returns every filament in the same order. *Verify: integration*
- **FR-010** — `POST /api/filaments` (editor) creates a filament: blank brand, material or color
  → 400 naming the field; an exact duplicate → 400 `duplicate`. *Verify: integration*
- **FR-011** — `PUT /api/filaments/{id}` (editor) — same validation, duplicate checked against
  *other* rows (saving a row unchanged is fine); missing → 404. *Verify: integration*
- **FR-012** — Creating or updating a filament registers its color if no color of that name
  exists, with the given hex or `#CCCCCC`; an existing color is never duplicated or changed.
  `GET /api/filament-colors` (open) lists colors by name. *Verify: integration*
- **FR-013** — `DELETE /api/filaments/{id}` (editor): 400 `has_spools` if any spool uses it,
  400 `has_profiles` if any print profile uses it; 404 if missing. *Verify: integration*

### Catalog sync

- **FR-020** — `POST /api/filaments/sync` (editor), and the startup auto-sync (002/FR-002),
  fetch the published catalog plus every additional source URL in settings (one per line,
  blanks ignored) and import each entry through FR-010's create path; invalid or duplicate
  entries are skipped, not fatal. Returns `{ ok, added, skipped }` and records the sync time.
  An additional source that fails is skipped. *Verify: integration*
- **FR-021** — If the published catalog can't be fetched (network error, non-2xx, or an
  unparseable body) the sync fails with an error and the sync time is **not** updated. Fetches
  are time-bounded. *Verify: integration* — **non-2xx/garbage treated as success (#175)**
- **FR-022** — The startup auto-sync runs only when there has never been a sync or the last one
  was more than 24 h ago, in the background, and is silent on failure. The manual sync is never
  throttled. *Verify: unit*
- **FR-023** — Catalog entries are read from the feed's PascalCase fields (`Brand`, `Material`,
  `Variant`, `Color`, `Hex`). *Verify: unit*

### Spools

- **FR-030** — `POST /api/spools` (editor) `{ filamentId, lotCode, purchasedAt, openedAt,
  emptiedAt, weightGrams, diameterMm, notes }` creates a spool; unknown filament → 400; dates
  out of order (opened before purchased, emptied before opened) → 400. *Verify: integration*
  — **not validated (#176)**
- **FR-031** — `GET /api/spools` (open) lists spools oldest-created first, each with its
  filament; `GET /api/spools/{id}` returns one, 404 if missing. *Verify: integration*
- **FR-032** — `PUT /api/spools/{id}` (editor) updates every field except the filament; 404 if
  missing. *Verify: integration*
- **FR-033** — `DELETE /api/spools/{id}` (editor): 400 `has_profiles` if a spool-specific
  profile uses it, 400 `has_prints` if a Print does; 404 if missing. *Verify: integration*

### Pages

- **FR-040** — `/filaments` searches by brand/material with paging, runs the catalog sync, and
  deletes with the refusal reason shown; `/filaments/new` and `/filaments/edit/[id]` edit the
  four fields with color autocomplete from known colors. *Verify: e2e*
- **FR-041** — `/spools` lists spools with their filament and status; `/spools/new` and
  `/spools/edit/[id]` pick the filament and edit the spool's fields. *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **#174 (fixed):** filament delete guard; **#175:** sync failure masking + no timeout; **#176:** spool
  validation; **#177:** no way to fix a color's hex (`#CCCCCC` placeholder is permanent).
- Spool "status" (Unopened / Opened / Empty) is derived from the dates — see 002/FR-020.
