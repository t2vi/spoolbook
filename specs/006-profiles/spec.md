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
  `source_preset_path`), `version_number`, `version_name`, `is_current_version`, `notes`.
- **Field spec** — the form definition: groups (tabs) → sections → fields, each with a key,
  label, unit, input kind, and current value as a string.

## Functional requirements

### Reading

- **FR-001** — `GET /api/profiles?filamentId=` (open) lists that filament's current versions,
  generic profiles before spool-specific ones, then by name. *Verify: integration*
- **FR-002** — `GET /api/profiles/inventory` (open) lists every current version by name, each
  with its filament (brand, material, variant, color), and `total`; no pagination (one page).
  *Verify: integration*
- **FR-003** — `GET /api/profiles/field-spec` (open) returns the form definition with empty
  values; `?profileId=` returns it pre-filled with that profile's name and values (booleans as
  `true`/`false`, blank for unset); unknown id → 404. *Verify: integration*

### Writing

- **FR-010** — `POST /api/profiles?filamentId=` (editor) `{ name, fields, spoolId?, source?,
  sourceSlicer?, rawSettingsJson? }` creates a profile; `fields` maps field keys to strings.
  Unknown filament, or a spool that doesn't exist or belongs to another filament → 400.
  *Verify: integration* — **filament/spool unchecked (#173)**
- **FR-011** — Validation, all reported together per field: blank name → `name: Name is
  required`; blank or non-integer `NozzleTempC` → error; any other non-blank value that doesn't
  parse as its type (whole number / number / true-false) → error naming the field. Blank
  values are stored as unset, never as 0. *Verify: integration*
- **FR-012** — `PUT /api/profiles/{id}` (editor) saves the same shape; provenance fields left
  out keep their stored values, and `notes` / `version_name` are never erased by a save that
  doesn't carry them. Missing id → 404. *Verify: integration* — **notes/version name erased
  (#172)**
- **FR-013** — `DELETE /api/profiles/{id}` (editor): 400 `has_prints` if any Print references
  it; 404 if missing. *Verify: integration*

### Versions

- **FR-020** — A profile version referenced by any Print is locked: updating it → 400 with
  `Locked`, and the stored settings never change. *Verify: integration*
- **FR-021** — Editing a profile produces a new current version (`version_number` + 1); the
  previous version stays as history and stops being current; lists show current versions only.
  This is also how a locked version is edited. *Verify: integration* — **not implemented
  (#171)**

### Import

- **FR-030** — `POST /api/profiles/import-preset` (editor, multipart `file`) auto-detects a
  sliced `.3mf` (reads `Metadata/project_settings.config`, already fully resolved) or a raw
  Bambu Studio preset `.json` (its `inherits` chain resolved against the system library, child
  values winning). Returns `{ ok, suggestedName, fields, rawSettingsJson }` to pre-fill the form
  — it never saves a profile. Errors: no file → 400; a zip without the settings entry → 400
  `no_project_settings`; neither zip nor JSON → 400 `invalid_file`; system library unreachable
  → 502. Remote calls are time-bounded. *Verify: integration* — **no timeout (#173)**
- **FR-031** — Imported values map Bambu keys to profile fields: first element of array
  values, Bambu booleans → `true`/`false`, `nil`/empty treated as absent, the literal `%` in
  percentage strings stripped. *Verify: integration*
- **FR-032** — `GET /api/profiles/system-presets` (open) lists the Bambu system filament
  preset names (sorted, unique) from slicer-service's bundled library; unreachable → 502.
  *Verify: integration*
- **FR-033** — `POST /api/profiles/system-presets/resolve` `{ name }` (editor) returns that
  preset's fully resolved fields (whole `inherits` chain); unknown name → 502 with an error.
  *Verify: integration*

### Pages

- **FR-040** — `/profiles` lists current profiles with their filament and delete (with the
  `has_prints` refusal shown). *Verify: e2e*
- **FR-041** — `/profiles/new` and `/profiles/edit/[id]` render the tabbed form from the field
  spec, pre-fill from a file import or a system preset, show per-field errors, and save.
  *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **#171:** the versioning model (FR-021) exists only in the schema; the backend never creates
  a second version. `profiles.rs`'s comment over `has_prints` ("Prints table doesn't exist yet")
  is stale — the check is live.
- **#172:** update erases `notes` / `version_name`.
- **#173:** filament/spool validation; no timeout on system-preset fetches.
- `notes`, `version_name` and `source_preset_path` can't be set from the API at all today.
