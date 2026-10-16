# 006 — Print profiles & Bambu preset import

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Profiles (#149)
**Code:** `spoolbook-rs/src/{profiles,profile_field_spec,bambu_import}.rs`; pages `/profiles`,
`/profiles/new`, `/profiles/edit/[id]` (`lib/components/profiles/ProfileForm.svelte`)

## Summary

A **Print profile** is a named set of ~135 Bambu Studio filament settings for one filament —
generic, or specific to one spool. Profiles are edited in a tabbed form mirroring Bambu
Studio's own layout, can be pre-filled from a sliced `.3mf`, a raw Bambu preset export, or a
Bambu system preset, and are what a Print records and a re-slice applies.

## Key entities

- **Print profile** (`print_profiles`) — `filament_id`, nullable `spool_id` (null = generic),
  `name`, `nozzle_temp_c` (the only required setting), every other setting nullable,
  provenance (`source` default `Manual`, `source_slicer`, `raw_settings_json`,
  `source_preset_path`), `version_number`, `version_name`, `is_current_version`,
  `previous_version_profile_id`, `notes`. Each row is one *version*. The versions linked
  through `previous_version_profile_id` form one profile's **version set**: working variants
  (e.g. "Winter 2026", "Fast draft") of which exactly one is **selected**
  (`is_current_version`) — the one lists, the Print modal and recommendations use.
- **Field spec** — the form definition: groups (tabs) → sections → fields, each with a key,
  label, unit, input kind, and current value as a string.

## Functional requirements

### Reading

- **FR-001** — `GET /api/profiles?filamentId=` (signed in) lists that filament's current versions,
  generic profiles before spool-specific ones, then by name. *Verify: integration*
- **FR-002** — `GET /api/profiles/inventory` (signed in) lists every selected version, each with
  its filament (brand, material, variant, color) and spool; filters by exact filament `brand` and
  `material` and by `scope` = `generic` (no spool) | `spool`; sorts by `sort` = `filament`
  (default: brand, then material, then oldest created) | `name` | `nozzleTemp` | `created` with
  `order` = `asc` (default) | `desc`; pages with `page` / `pageSize` (defaults 1 / 20;
  non-positive → default); returns `{ profiles, total, page, pageSize, totalPages ≥ 1 }`.
  *Verify: integration*
- **FR-003** — `GET /api/profiles/field-spec` (signed in) returns the form definition with empty
  values; `?profileId=` returns it pre-filled with that profile's name and values (booleans as
  `true`/`false`, blank for unset) plus its `versionNumber`, `isCurrentVersion`, `filamentId`
  and `spoolId` (null = generic); unknown id → 404. *Verify: integration*

### Writing

- **FR-010** — `POST /api/profiles?filamentId=` (editor) `{ name, fields, spoolId?, source?,
  sourceSlicer?, rawSettingsJson? }` creates a profile; `fields` maps field keys to strings.
  Field-keyed 400s, nothing saved: unknown filament → `filamentId: Unknown filament.`; a spool
  that doesn't exist → `spoolId: Unknown spool.`; a spool of another filament → `spoolId: That
  spool is a different filament.` The spool rule also applies to update (FR-012) and new
  versions (FR-021), against the profile's own filament. *Verify: integration*
- **FR-011** — Validation, all reported together per field: blank name → `name: Name is
  required`; blank or non-integer `NozzleTempC` → error; any other non-blank value that doesn't
  parse as its type (whole number / number / true-false) → error naming the field. Blank
  values are stored as unset, never as 0. *Verify: integration*
- **FR-012** — `PUT /api/profiles/{id}` (editor) saves the same shape; provenance fields left
  out keep their stored values, and `notes` / `version_name` are never erased by a save that
  doesn't carry them. Missing id → 404. *Verify: integration*
- **FR-013** — `DELETE /api/profiles/{id}` (editor): 400 `has_prints` if any Print references
  it; 400 `has_newer_version` if a later version builds on it; 404 if missing. Deleting the
  selected version selects its previous version. *Verify: integration*

### Versions

- **FR-020** — A profile version referenced by any Print is locked: updating it → 400 with
  `Locked`, and the stored settings never change. *Verify: integration*
- **FR-021** — Editing a version no Print has used changes it in place (no new version).
  `POST /api/profiles/{id}/versions` (editor, same body as update) saves the edits as a new
  version instead: a copy of `{id}` with the edits applied — the setting fields, name and spool
  come from the request; provenance (`source`, `source_slicer`, `raw_settings_json`,
  `source_preset_path`) and notes always come from `{id}` (any provenance in the request is
  ignored: a new version is the same profile edited, not a re-import); version name cleared;
  `version_number` = the highest number in its version set + 1,
  `previous_version_profile_id = {id}`, selected; every other version in the set is
  unselected. All or nothing. Any version can be the starting point, selected or not; missing
  → 404; validation as FR-011. This is how a locked version is edited. *Verify: integration*
- **FR-022** — Lists (FR-001, FR-002) show the selected version of each set only; a Print keeps
  pointing at the exact version it used. *Verify: integration*
- **FR-023** — `GET /api/profiles/{id}/versions` (signed in) lists every version in `{id}`'s
  version set, highest number first, each with id, `versionNumber`, `versionName`,
  `nozzleTempC`, created time, `selected`, and `locked` (used by any Print); 404 if missing.
  *Verify: integration*
- **FR-024** — `POST /api/profiles/{id}/select` (editor) makes `{id}` the selected version of
  its set and unselects the others, atomically; selecting the already-selected one is a no-op;
  404 if missing. *Verify: integration*
- **FR-025** — `PUT /api/profiles/{id}/version-name` `{ versionName }` (editor) sets (trimmed) or
  clears (blank) a version's name. Allowed on locked versions too — it's a label, not a
  setting. 404 if missing. *Verify: integration*
- **FR-026** — `POST /api/profiles/{id}/duplicate` (editor) creates a new, independent profile
  — its own version set, version 1, selected — copying every setting, the filament, spool,
  provenance and notes of `{id}`, named "`<name>` (copy)" with no version name; returns
  `{ ok, profile }`; 404 if missing. *Verify: integration*

### Import

- **FR-030** — `POST /api/profiles/import-preset` (editor, multipart `file`) auto-detects a
  sliced `.3mf` (reads `Metadata/project_settings.config`, already fully resolved) or a raw
  Bambu Studio preset `.json` (its `inherits` chain resolved against the system library, child
  values winning). Returns `{ ok, suggestedName, fields, rawSettingsJson }` to pre-fill the form
  — it never saves a profile. Errors: no file → 400; a zip without the settings entry → 400
  `no_project_settings`; neither zip nor JSON → 400 `invalid_file`; system library unreachable
  → 502. Every call to slicer-service gives up after 30 s (→ 502). *Verify: integration*
- **FR-031** — Imported values map Bambu keys to profile fields: first element of array
  values, Bambu booleans → `true`/`false`, `nil`/empty treated as absent, the literal `%` in
  percentage strings stripped. *Verify: integration*
- **FR-032** — `GET /api/profiles/system-presets` (signed in) lists the Bambu system filament
  preset names (sorted, unique) from slicer-service's bundled library; unreachable → 502.
  *Verify: integration*
- **FR-033** — `POST /api/profiles/system-presets/resolve` `{ name }` (editor) returns that
  preset's fully resolved fields (whole `inherits` chain); unknown name → 502 with an error.
  *Verify: integration*

### Pages

- **FR-040** — `/profiles` lists profiles (FR-002) with columns name, filament (with swatch,
  007/FR-051), scope ("Generic" or the spool's lot code), nozzle temp, source and selected
  version (number and name); filters for brand, material and scope; sortable filament / name /
  nozzle temp / created headers; paging with a page-size choice (10 / 20 / 50); filters, sort
  and page in the URL; delete with the `has_prints` / `has_newer_version` refusal shown.
  *Verify: e2e*
- **FR-041** — `/profiles/new` and `/profiles/edit/[id]` render the tabbed form from the field
  spec, pre-fill from a file import or a system preset, show per-field errors, and save. A
  Spool picker chooses generic or one of the profile's filament's spools; the edit page starts
  on the profile's current choice and saving sends it. The
  edit page shows the version number and a "Save as new version" button (FR-021); a `Locked`
  refusal points the user at it. Saving from the edit page keeps the profile's provenance
  (`source`, `source_slicer`, `raw_settings_json`) unless a file or system preset was imported
  into the form during that edit — then the import's provenance is saved. The filament picker is
  searchable (007/FR-052). Unsaved edits are guarded (002/FR-008). *Verify: e2e*
- **FR-042** — The edit page has a Versions panel (FR-023): each version's number, name, nozzle
  temp, created date and Selected / Locked badges; open any version in the editor; "Use this
  version" selects it (FR-024); rename sets its version name (FR-025). *Verify: e2e*
- **FR-043** — The edit page has "Duplicate" (FR-026), which opens the new copy in the editor.
  *Verify: e2e*

## Notes

- **Lost-features review (2026-10-16):** FR-002, 023–026 and 040–043 restore the desktop app's
  profile list filters/sorting/scope, version picker and naming, duplicate and unsaved-changes
  warning. Versions are now working variants you choose between (FR-024), not just history; so
  any version can start a new one (FR-021, was current-only). Dropped on purpose: linking a
  profile to a local Bambu preset file and "Check for updates" — a server app has no local
  preset folder.

- Retroactive: behavior read from the code at `3fdca27`.
- **#171 (fixed):** versioning existed only in the schema. Versions now chain through
  `previous_version_profile_id`, mirroring Projects. The old desktop app grouped versions by
  the local preset file path instead — meaningless once profiles stopped coming from local files.
- **#172 (fixed):** update used to erase `notes` / `version_name`.
- **#173 (fixed):** unknown filament/spool used to 500, a profile could point at another
  filament's spool, and system-preset fetches had no timeout.
- **#183 (fixed):** the edit page always sent `source: Manual`, relabelling every imported
  profile on save.
- **#184 (fixed):** the form had no spool choice and always sent `spoolId: null`, so saving a
  spool-specific profile made it generic.
- `notes`, `version_name` and `source_preset_path` can't be set from the API at all today.
