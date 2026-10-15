# 004 — Printers: configuration, live status, control, send print, camera

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Printers & telemetry (#151)
**Code:** `spoolbook-rs/src/{printers,send_print,printer_camera,bambu_mqtt_payload_parser,jpeg_frame_extractor}.rs`;
pages `/printers`, `/printers/new`, `/printers/edit/[id]` (`lib/components/printers/{PrinterCard,PrintModal,PrinterForm}.svelte`)

## Summary

Configuring Bambu printers (LAN mode), showing each one's live state, pausing/resuming/stopping
it, sending a sliced project plate to it, and relaying its camera to the browser. The
background telemetry connection itself is spec 001.

## Key entities

- **Printer** (`printers`) — unique `name`, `model`, and LAN connection details `ip_address`,
  `access_code`, `serial_number` (all three needed for anything live).
- **Live snapshot** — `connected`, `amsUnits`, `gcodeState`, `errors`, `cameraStatus`
  (`NotStarted` | `Connecting` | `Streaming` | `Unavailable`), `cameraError`.

## Functional requirements

### Configuration

- **FR-001** — `GET /api/printers` (open) lists Printers ordered by name.
  *Verify: integration*
- **FR-002** — A Printer's access code is write-only: no API response ever contains it.
  Printers carry `hasAccessCode` (true when one is stored) instead. *Verify: integration*
- **FR-003** — `POST /api/printers` / `PUT /api/printers/{id}` (editor) save name, model, IP,
  access code, serial. Blank name → 400 `Name is required`; a name already used by another
  Printer → 400 `duplicate`; update of a missing id → 404. On update, a blank or omitted
  access code keeps the stored one. Saving (re)starts the Printer's telemetry loop
  (001/FR-002). *Verify: integration*
- **FR-004** — `DELETE /api/printers/{id}` (editor) → 400 `has_prints` if any Print references
  it; 404 if missing; otherwise deletes it, its telemetry Jobs/Readings, and stops its loop.
  *Verify: integration*
- **FR-005** — `POST /api/printers/test` `{ ipAddress, accessCode?, printerId? }` (editor)
  connects once to the printer's broker — with `accessCode`, or when that's blank the stored
  code of `printerId`; neither available → 400 `Access code is required`. Reports
  `{ ok: true }` on a successful handshake, or `{ ok: false, error }` — a timeout after 6 s names the IP / LAN-mode as the likely cause.
  *Verify: integration*

### Live status & control

- **FR-010** — `GET /api/printers/{id}/live` (open) is a server-sent-event stream emitting the
  Printer's live snapshot every 2 s; a Printer with no live connection reports
  `connected: false`. *Verify: integration*
- **FR-011** — Status reports are parsed for `gcode_state`, `task_id`, nozzle/bed/chamber
  temperature, progress, layer numbers, and the full AMS inventory (per unit humidity: the
  real % when the AMS reports `humidity_raw`, else only the coarse 1–5 level). A report with no
  `print` object, no `gcode_state`, or invalid JSON is ignored. `RUNNING`, `PAUSE` and
  `PREPARE` are active states; everything else is terminal/idle. *Verify: unit*
- **FR-012** — The printer's blocking `print_error` and HMS codes are decoded into
  human-readable messages (known codes get specific advice; unknown codes a generic message,
  never a bare number); no error → empty list. *Verify: unit*
- **FR-013** — `POST /api/printers/{id}/control` `{ command }` (editor) accepts `pause`,
  `resume`, `stop` and publishes it on the Printer's live connection. Unknown Printer or one
  with no serial → 404; other command → 400 `Unknown command.`; no live connection → 400 with
  an error saying it isn't connected. *Verify: integration*

### Sending a print

- **FR-030** — `POST /api/printers/{id}/print` `{ projectId, platerId, spoolId, profileId,
  useAms, amsSlot }` (editor). Missing printer → 404; printer without IP/access code/serial →
  400; missing project → 404; unknown profile → 400 `Unknown print profile.`; unknown spool →
  400 `Unknown spool.` — all checked before anything is uploaded or sent. *Verify: integration*
- **FR-031** — The plate's `Metadata/plate_<platerId>.gcode` must exist in the project's
  `.3mf` (else 400 naming the entry — "is this a sliced export?"); its MD5 (of that gcode entry,
  not the whole file) is sent with the command. *Verify: integration*
- **FR-032** — The `.3mf` is uploaded over implicit FTPS (port 990, `bblp` + access code)
  under a printer-safe name: the display name's stem reduced to `[A-Za-z0-9_-]`, at most 60
  chars, `print` if nothing survives, plus `.3mf`. Then a `project_file` command starts it:
  auto bed leveling, no timelapse, vibration calibration off for P2S models only, a fresh
  numeric submission id, and — when `useAms` — an AMS mapping to the chosen global tray
  (`amsSlot`); no mapping otherwise. Upload or publish failure → 400 with the cause.
  *Verify: unit, live*
- **FR-033** — On success the `InProgress` Print is created immediately (profile, spool,
  printer, project, plate, start = now), so the telemetry Job attaches to it (001/FR-015).
  *Verify: integration*
- **FR-034** — With `SPOOLBOOK_MQTT_DEBUG` set, the exact command sent is logged.
  *Verify: live*

### Camera

- **FR-040** — `GET /printers/{id}/camera` (editor only — a view into physical space) streams
  the printer's RTSPS camera as MJPEG (`multipart/x-mixed-replace`). Missing printer or no
  IP/access code → 404. *Verify: integration*
- **FR-041** — One camera connection per printer, shared by every viewer; it starts on the
  first viewer and stops 10 s after the last leaves. Status moves `NotStarted` → `Connecting`
  → `Streaming`; it gives up as `Unavailable` (with ffmpeg's last error) after 3 failed
  attempts before any frame, or 10 after frames had flowed. *Verify: integration*
- **FR-042** — `POST /api/printers/{id}/camera/retry` (editor) resets an `Unavailable` camera to
  `NotStarted` (no-op otherwise or when no camera exists yet). *Verify: integration*
- **FR-043** — Changing a printer's IP or access code, or deleting it, drops its camera
  connection — open viewers' streams end — so the next view (or bed photo) uses the current
  details. Saving a printer without changing either leaves the camera alone.
  *Verify: integration*
- **FR-044** — At the end of an attached print (001/FR-022) one still frame is captured
  (waiting at most 8 s) and stored as the Print's bed photo; no frame → left empty.
  *Verify: integration*
- **FR-045** — The relay terminates TLS (1.2) itself and rewrites RTSP addresses so ffmpeg only
  ever talks to the local proxy: only the request line of client requests, and only the header
  block of responses (`rtsps://` → `rtsp://`, host[:port] → proxy), never the body. JPEG frames
  are cut from ffmpeg's output on SOI/EOI markers across chunk boundaries. *Verify: unit*

### Pages

- **FR-050** — `/printers` shows a card per printer: connection + print state, decoded errors,
  AMS trays, live camera (with retry / pop-out), pause/resume/stop, its 5 most recent Prints,
  and a Print modal (upload or pick a project plate, spool, profile with a recommended
  default, AMS tray, optional re-slice) that sends the print. *Verify: e2e*
- **FR-051** — `/printers/new` and `/printers/edit/[id]` edit name, model (dropdown), IP,
  access code, serial, with a Test Connection button; delete is offered from the card.
  *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **#165 (fixed):** access codes used to leak through the open `GET /api/printers`, and camera
  retry wasn't editor-gated.
- **#166 (fixed):** send print used to start the job before checking profile/spool, then fail to
  record the Print.
- **#167 (fixed):** the camera kept using a printer's old IP/access code until restart (FR-043).
- The ffmpeg pipeline and FTPS upload are only provable against a real printer
  (`tests/printer_live.rs`); the camera relay was written without real-hardware verification.
- Only RTSP-camera models (X1/X2/H2/P2 family) are supported; A1/P1's port-6000 image protocol
  is not implemented.
