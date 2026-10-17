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
- **Filament color** (`filament_colors`) — unique `name`, `hex`: one `#RRGGBB`, or a
  comma-separated list of them for a multi-color filament (e.g. `#1A1A1A, #D4AF37` for
  black + gold), and whether a person set that hex (`hex_set_by_user`). Filaments refer to a
  color by name.
- **Spool** (`spools`) — `filament_id`, `lot_code`, `purchased_at`, `opened_at`, `emptied_at`,
  `weight_grams`, `diameter_mm`, `notes`.

## Functional requirements

### Filaments

- **FR-001** — `GET /api/filaments` (signed in) filters by exact `brand` and `material`, ordered by
  brand then material, paged (`page` default 1, `pageSize` default 20; non-positive → default),
  returning `{ entries, total, page, pageSize, totalPages ≥ 1 }`. `GET /api/filaments/all`
  (signed in) returns every filament in the same order. *Verify: integration*
- **FR-010** — `POST /api/filaments` (editor) creates a filament: blank brand, material or color
  → 400 naming the field; an exact duplicate → 400 `duplicate`. *Verify: integration*
- **FR-011** — `PUT /api/filaments/{id}` (editor) — same validation, duplicate checked against
  *other* rows (saving a row unchanged is fine); missing → 404. *Verify: integration*
- **FR-012** — Creating or updating a filament registers its color if no color of that name
  exists, with the given hex or `#CCCCCC`; an existing color is never duplicated, and its hex
  changes only through the Colors screen (FR-015) or the catalog sync (FR-027).
  `GET /api/filament-colors` (signed in) lists colors by name. *Verify: integration*
- **FR-013** — `DELETE /api/filaments/{id}` (editor): 400 `has_spools` if any spool uses it,
  400 `has_profiles` if any print profile uses it; 404 if missing. *Verify: integration*

### Colors

- **FR-014** — A color's hex is one or more `#RRGGBB` values (case-insensitive), separated by
  commas; spaces around each part are ignored and empty parts are dropped. Anything else is
  invalid. *Verify: integration*
- **FR-015** — `POST /api/filament-colors` `{ name, hex }` (editor) creates a color; `PUT
  /api/filament-colors/{id}` (editor) updates one. Name and hex are trimmed. Blank name → 400
  `Name is required`; invalid hex (FR-014) → 400 `invalid_hex`; a name another color already
  has → 400 `duplicate`; update of a missing id → 404. Renaming a color renames it on every
  filament that uses it, in the same transaction, so their swatches keep working. A color
  created or updated here has its hex marked as set by a person (FR-027). *Verify: integration*
- **FR-016** — `DELETE /api/filament-colors/{id}` (editor): 400 `in_use` while any filament uses
  that color's name; 404 if missing. *Verify: integration*
- **FR-017** — `GET /api/filament-colors/inventory` (signed in) filters by `q` (case-insensitive
  substring of the name), sorts by `sort` = `name` (default) | `hex` with `order` = `asc`
  (default) | `desc`, pages with `page` / `pageSize` (defaults 1 / 20; non-positive → default),
  and returns `{ colors, total, page, pageSize, totalPages ≥ 1 }`. `GET /api/filament-colors`
  (FR-012) still returns the full list, for swatches and pickers. *Verify: integration*

### Catalog sync

- **FR-020** — `POST /api/filaments/sync` (editor), and the startup auto-sync (002/FR-002),
  fetch the published catalog plus every additional source URL in settings (one per line,
  blanks ignored) and import each entry through FR-010's create path; invalid or duplicate
  entries are skipped, not fatal. Returns `{ ok, added, skipped }` and records the sync time.
  An additional source that fails is skipped. *Verify: integration*
- **FR-021** — If the published catalog can't be fetched (network error, non-2xx, or a body
  that isn't a JSON catalog — an empty list is fine) the sync fails with 400 `{ ok: false,
  error }` and the sync time is **not** updated. Every catalog fetch (published or additional)
  gives up after 30 s. *Verify: integration*
- **FR-022** — The startup auto-sync runs only when there has never been a sync or the last one
  was more than 24 h ago, in the background, and is silent on failure. The manual sync is never
  throttled. *Verify: unit*
- **FR-023** — Catalog entries are read from the feed's PascalCase fields (`Brand`, `Material`,
  `Variant`, `Color`, `Hex`). `Hex` is null, one `#RRGGBB`, or a comma-separated list of them
  for a multi-color filament (FR-014), in the order the colors appear. *Verify: unit*
- **FR-024** — `FILAMENT_CATALOG_URL`, when set, replaces the published catalog URL for syncing
  and for the URL Settings shows (used by tests). *Verify: integration*
- **FR-025** — An imported entry whose material or variant contains HTML markup is cleaned
  before FR-010's create path: material and variant are joined, tags (`<…>`) removed, whitespace
  collapsed, and the result re-split at the first space — material is the first word, variant
  the rest (none if nothing is left). So `<span` + `class="align-middle">PLA Matte</span>`
  imports as material `PLA`, variant `Matte`. *Verify: unit*
- **FR-026** — At startup, every stored filament whose material or variant contains markup is
  repaired the same way (FR-025). If no other filament has the cleaned brand / material /
  variant / color, it's renamed in place; otherwise its spools and profiles move to that
  filament and the broken one is deleted, all in one transaction. Nothing that references a
  filament is lost. *Verify: integration*
- **FR-027** — The catalog sync keeps color hexes current. When an entry gives a valid hex
  (FR-014) for a color that already exists and whose hex wasn't set by a person (FR-015), the
  color takes the entry's hex — so a feed that now knows a multi-color filament's colors updates
  its existing swatch. A hex set by a person is never overwritten; an entry without a hex changes
  nothing. Colors that already held a list of two or more hexes before this rule existed count as
  set by a person, so a deliberate list is never replaced. *Verify: integration*

### Spools

- **FR-030** — `POST /api/spools` (editor) `{ filamentId, lotCode, purchasedAt, openedAt,
  emptiedAt, weightGrams, diameterMm, notes }` creates a spool; unknown filament → 400
  `Unknown filament.`; dates out of order → 400 `Dates are out of order — purchased, then
  opened, then emptied.` Dates are compared by calendar day, for every pair that's present
  (purchased ≤ opened ≤ emptied, and purchased ≤ emptied). *Verify: integration*
- **FR-031** — `GET /api/spools` (signed in) lists spools oldest-created first, each with its
  filament; `GET /api/spools/{id}` returns one, 404 if missing. *Verify: integration*
- **FR-032** — `PUT /api/spools/{id}` (editor) updates every field except the filament, with
  FR-030's date-order rule; 404 if missing. *Verify: integration*
- **FR-033** — `DELETE /api/spools/{id}` (editor): 400 `has_profiles` if a spool-specific
  profile uses it, 400 `has_prints` if a Print does; 404 if missing. *Verify: integration*
- **FR-034** — `GET /api/spools/inventory` (signed in) filters by exact filament `brand` and
  `material`, `status` (`Unopened` = no opened date, `Opened` = opened but not emptied, `Empty`
  = emptied) and `purchasedFrom` / `purchasedTo` (inclusive calendar days); sorts by `sort` =
  `filament` (default: brand, then material, then oldest created) | `purchasedAt` | `openedAt` |
  `emptiedAt` with `order` = `asc` (default) | `desc` — spools without that date always last;
  pages with `page` / `pageSize` (defaults 1 / 20); returns `{ spools, total, page, pageSize,
  totalPages ≥ 1 }`, each spool with its filament. *Verify: integration*

### Swatches & pickers

- **FR-050** — A color swatch draws its hex (FR-014): one value fills it solid; several split it
  into equal diagonal wedges from top-left to bottom-right, hard edges, in the order listed (so
  black + gold reads as two flat halves, not a blend). Its border turns light when the colors'
  average luminance (0.299 R + 0.587 G + 0.114 B, over 255) is under 0.3, so near-black swatches
  stay visible. A color with no known hex shows an empty outlined swatch. *Verify: e2e*
- **FR-051** — A filament is shown with its color's swatch wherever it's listed or picked: the
  filament list, spool list, profile list, print list, print detail, the Print modal's spool
  picker, every filament picker, and the filament form (updating live as the color is typed).
  *Verify: e2e*
- **FR-052** — Every filament picker (spool form, profile form) is a type-to-search box: typing
  matches brand, material, variant and color as case-insensitive substrings (all typed words
  must match), options show the swatch, and it is fully keyboard-operable (arrows, Enter,
  Escape). A picker never renders the whole catalog as one long list. *Verify: e2e*

### Pages

- **FR-040** — `/filaments` searches by brand/material with paging, runs the catalog sync
  (showing the added/skipped counts, or the error when it fails), and
  deletes with the refusal reason shown; the add / edit filament modal (002/FR-015) edits the
  four fields with color autocomplete from known colors. *Verify: e2e*
- **FR-041** — `/spools` lists spools (FR-034) with columns brand, material, color (swatch),
  lot code, purchased, opened, emptied and status; filters for brand, material, status and
  purchase-date range; sortable filament / date headers; paging with a page-size choice
  (10 / 20 / 50); filters, sort and page in the URL. The add / edit spool modal (002/FR-015)
  picks the filament (FR-052) and edits the spool's fields. *Verify: e2e*
- **FR-042** — `/filaments/colors` (sidebar: Filaments → Colors) lists colors (FR-017) with
  swatch, name and hex; a name filter; sortable name / hex; paging with a page-size choice;
  add and edit in a modal (002/FR-015) with name, hex (hint: comma-separate for multi-color) and a live
  swatch preview, showing `duplicate` / `invalid_hex` errors; delete after confirmation, showing
  the `in_use` refusal. *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **#174 (fixed):** filament delete guard; **#175 (fixed):** sync failure masking + no timeout; **#176 (fixed):** spool
  validation; **#177:** no way to fix a color's hex — now FR-015/042.
- **Lost-features review (2026-10-16):** FR-014–017, 034, 041–042 and 050–052 restore what the
  retired desktop app had (multi-color gradient swatches, the Colors screen, swatches in lists
  and pickers, searchable filament pickers, the spool list's columns/filters/sorting) and the
  web UI never got.
- Spool "status" (Unopened / Opened / Empty) is derived from the dates — see 002/FR-020.
