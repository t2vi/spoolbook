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
  `previous_version_profile_id`, `notes`. Each row is one *version*.
- **Field spec** — the form definition: groups (tabs) → sections → fields, each with a key,
  label, unit, input kind, and current value as a string.

## Functional requirements

### Reading

- **FR-001** — `GET /api/profiles?filamentId=` (signed in) lists that filament's current versions,
  generic profiles before spool-specific ones, then by name. *Verify: integration*
- **FR-002** — `GET /api/profiles/inventory` (signed in) lists every current version by name, each
  with its filament (brand, material, variant, color), and `total`; no pagination (one page).
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
  current version makes its previous version current again. *Verify: integration*

### Versions

- **FR-020** — A profile version referenced by any Print is locked: updating it → 400 with
  `Locked`, and the stored settings never change. *Verify: integration*
- **FR-021** — Editing a version no Print has used changes it in place (no new version).
  `POST /api/profiles/{id}/versions` (editor, same body as update) saves the edits as a new
  version instead: a copy of `{id}` with the edits applied — the setting fields, name and spool
  come from the request; provenance (`source`, `source_slicer`, `raw_settings_json`,
  `source_preset_path`) and notes always come from `{id}` (any provenance in the request is
  ignored: a new version is the same profile edited, not a re-import); version name cleared;
  `version_number` + 1, `previous_version_profile_id = {id}`, current;
  `{id}` stops being current. All or nothing. Only the current version can be branched — an
  older one → 400 `not_current_version`; missing → 404; validation as FR-011. This is how a
  locked version is edited. *Verify: integration*
- **FR-022** — Lists (FR-001, FR-002) show current versions only; a Print keeps pointing at the
  exact version it used. *Verify: integration*

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

- **FR-040** — `/profiles` lists current profiles with their filament and delete (with the
  `has_prints` refusal shown). *Verify: e2e*
- **FR-041** — `/profiles/new` and `/profiles/edit/[id]` render the tabbed form from the field
  spec, pre-fill from a file import or a system preset, show per-field errors, and save. A
  Spool picker chooses generic or one of the profile's filament's spools; the edit page starts
  on the profile's current choice and saving sends it. The
  edit page shows the version number and a "Save as new version" button (FR-021); a `Locked`
  refusal points the user at it. Saving from the edit page keeps the profile's provenance
  (`source`, `source_slicer`, `raw_settings_json`) unless a file or system preset was imported
  into the form during that edit — then the import's provenance is saved. *Verify: e2e*

## Notes

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
