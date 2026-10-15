# 005 — Projects: upload, plates, versions, re-slicing

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Projects (#152)
**Code:** `spoolbook-rs/src/{projects,project_upload,reslicing,profile_config_patcher}.rs`;
pages `/projects`, `/projects/[id]`; upload + re-slice also from the Print modal (004/FR-050)

## Summary

A **Project** is a stored Bambu Studio `.3mf`. Projects are uploaded (or fetched from a URL),
their plates read for printing, and re-sliced with a chosen print profile through the separate
`slicer-service`. A re-slice never overwrites a Project: the result becomes the next version in
a chain, and only current versions are listed.

## Key entities

- **Project** (`projects`) — `file_path` (content-hash name under project storage),
  `file_name` (display name), `mesh_hash` (SHA-256 of the `3D/3dmodel.model` entry),
  `last_known_write_time_utc`, `last_known_file_size_bytes`, `previous_version_project_id`,
  `version_number` (from 1), `is_current_version`.
- **Plate** — read from the `.3mf` on demand: `platerId`, `platerName`, base64 thumbnail.
- **Project storage** — the `projects/` directory next to the SQLite file (so it lives on the
  same persistent volume).

## Functional requirements

### Storing projects

- **FR-001** — Project files are stored in `projects/` beside the DB file
  (`SPOOLBOOK_DB_PATH`'s directory, or the working directory for a bare `dev.db`), created if
  missing. *Verify: unit*
- **FR-002** — `POST /api/projects/upload` (editor, multipart `file`, up to 100 MB) stores the
  bytes as `<sha256>.3mf` and returns `{ ok, project, created }`. Uploading identical bytes
  again returns the existing Project (`created: false`) instead of a duplicate. No `file` field
  → 400; bytes that aren't a zip archive (as every `.3mf` is) → 400 `Not a .3mf file.`, nothing
  stored. *Verify: integration*
- **FR-003** — `POST /api/projects/import-url` `{ url }` (editor) downloads a direct link and
  stores it exactly like an upload (including the zip check). Non-http(s) or malformed URL →
  400; failed/non-2xx fetch, empty body, or over 100 MB → 400 with the reason. The download
  gives up after 2 minutes and stops as soon as it passes 100 MB. Display name is the URL's
  last path segment if it ends in `.3mf`, else `download.3mf`. *Verify: integration*
- **FR-004** — A new Project records its size, import time, and mesh hash (none if the file has
  no readable model entry). *Verify: integration*

### Reading

- **FR-010** — `GET /api/projects` (open) lists current versions only, by display name.
  `GET /api/projects/{id}` (open) returns any version, 404 if missing. *Verify: integration*
- **FR-011** — `GET /api/projects/{id}/plates` (open) reads the file's
  `Metadata/model_settings.config` and returns each plate's id, name (blank → null) and
  thumbnail; an unreadable/non-3mf file → empty list; unknown Project → 404.
  *Verify: integration*
- **FR-012** — `PUT /api/projects/{id}` `{ fileName }` (editor) renames (trimmed); blank → 400;
  missing → 404. *Verify: integration*
- **FR-013** — `DELETE /api/projects/{id}` (editor): 400 `has_prints` when any Print
  references it; 400 `has_newer_version` if a later version builds on it; 404 if missing.
  Deleting the current version makes its previous version current again (so the history stays
  listed). Its stored file is removed once no other Project uses the same file.
  *Verify: integration*

### Versions

- **FR-020** — `POST /api/projects/{id}/link-version` `{ previousVersionProjectId }` (editor)
  makes `{id}` the next version: the predecessor stops being current, `{id}` becomes current
  with `version_number` = predecessor's + 1, atomically. Unknown id → 404. Linking a Project to
  itself, or to a predecessor that already descends from it → 400 `would_create_cycle`.
  *Verify: integration*
- **FR-021** — `GET /api/projects/version-candidate?meshHash=&fileName=&excludeProjectId=`
  (open) suggests an existing Project as the predecessor: same mesh hash first, else same
  display name; `null` when nothing matches. *Verify: integration*

### Re-slicing

- **FR-030** — `POST /api/projects/{id}/reslice` `{ profileId }` (editor) copies the `.3mf`
  (original untouched), replaces its `Metadata/project_settings.config` with one patched from
  the profile (FR-031), posts it to `slicer-service` (`RESLICE_SERVICE_URL`, default
  `http://localhost:8100`, `POST /slice`), and stores the result as a Project. Missing project
  or profile → 404; no settings entry → 400 ("not a sliced export?"); slicer-service error →
  400 with its status and detail. The temporary copy is always deleted. The slicer call
  gives up after 10 minutes. *Verify: integration*
- **FR-031** — Patching sets each non-null profile field on its Bambu Studio key: array-valued
  keys get index 0 only (other elements kept), booleans become `"0"`/`"1"`; null profile fields
  and keys the profile doesn't own are left as they were. *Verify: unit*
- **FR-032** — A newly created re-slice result is chained as the next version of the source
  Project (FR-020); a result byte-identical to an existing Project isn't re-chained.
  *Verify: integration*

### Pages

- **FR-040** — `/projects` lists current Projects with rename and (multi-select) delete, with
  confirmation and the `has_prints` refusal shown. *Verify: e2e*
- **FR-041** — `/projects/[id]` shows the Project's version badge (superseded or current), a
  link to its previous version, its plates with thumbnails, and its Prints. *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **#168 (fixed):** deleting a superseded version failed (500), deleting the current one hid its
  whole history, and the stored file was never removed. Same rule as print profiles (006/FR-013).
- **#169 (fixed):** upload accepted non-.3mf bytes; link-version allowed self/cycles; URL import
  and re-slice had no timeouts; URL import checked size only after the full download. Linking
  onto a non-current version stays allowed (re-slicing an older version branches from it).
- **#170:** URL import and manual version linking (FR-003, FR-020–021) have no UI.
- `projects.rs:15`'s comment calling re-slicing out of scope is stale (#156).
- `last_known_write_time_utc` is the import time, not the file's mtime — nothing reads it for
  drift detection.
