# Changelog

| Version | Date | Notes |
|---|---|---|
| [v1.4.3](docs/releases/v1.4.3.md) | 2026-08-30 | Fixed sending a print failing with "Failed to get AMS mapping table" (07FF-8012) — a re-sliced `.3mf` has no AMS data, so the print now always feeds from the threaded filament; fixed live status stuck on "Not connected" / "No live job data yet" (MQTT packet-size limit was smaller than a P2S status dump, causing a silent connection flap); fixed a DB error during a print killing telemetry and the pause/resume/stop buttons; fixed `stop`/`pause`/`resume` sometimes ignored (non-numeric command id); camera on the printer card is now pop-out only (the always-on feed starved the printer's own camera) |
| [v1.4.2](docs/releases/v1.4.2.md) | 2026-08-29 | Fixed the live camera feed failing on every Docker install (`ffmpeg` was missing from the container image); fixed live telemetry showing "No live job data yet" during a print (never requested a full status dump on connect); fixed the "Docs site" CI workflow being invalid since the template sync |
| [v1.4.1](docs/releases/v1.4.1.md) | 2026-08-29 | Fixed a printer added or edited through the UI never starting its live-telemetry connection until the next backend restart (Test connection worked, but the card stayed "Not connected" and Print failed) — mostly affected Docker installs |
| [v1.4.0](docs/releases/v1.4.0.md) | 2026-08-26 | Export/import to migrate data between spoolbook installs (merges, doesn't replace) from the Settings page; fixed uploaded `.3mf` files being lost on every redeploy (were stored in temp, not the persistent volume) |
| [v1.3.0](docs/releases/v1.3.0.md) | 2026-08-26 | Bed photo auto-captured at print end (replacing the plate thumbnail on print detail); outdoor weather and mid-print chamber temp/AMS humidity/layer progress charted hour-by-hour on the print detail page; print detail layout reworked (wider, tabbed charts); fixed prints getting stuck "In Progress" after a backend restart mid-print |
| [v1.2.0](docs/releases/v1.2.0.md) | 2026-08-25 | Sidebar navigation replacing the top header; real Projects page (list/rename/delete/detail view, version chaining on re-slice); Prints made view-only (manual log/edit form removed); full shadcn-svelte restyle; AMS humidity now shows the real percentage instead of a misleading coarse index |
| [v1.1.0](docs/releases/v1.1.0.md) | 2026-08-25 | Sending a print to a real printer confirmed working end-to-end for the first time (FTPS + MQTT fixes); real user accounts with Google sign-in; ambient weather auto-fetch; Bambu Studio preset import; printer model dropdown; profile save bug fixed |
| [v1.0.0](docs/releases/v1.0.0.md) | 2026-08-25 | Entire backend rewritten in Rust; .NET/EF Core deleted outright; real auth bug fixed (SvelteKit had no working login without .NET); manual DB migration required for existing installs |
| [v0.1.8](docs/releases/v0.1.8.md) | 2026-08-15 | One-click Docker install (spoolbook + BambuStudio slicer service via GHCR); Avalonia desktop UI fully retired; repo made public |
| [v0.1.7](docs/releases/v0.1.7.md) | 2026-08-14 | Blazor Server retired — SvelteKit is now the web UI; Recommend page removed; printer controls gate on live print state |
| [v0.1.6](docs/releases/v0.1.6.md) | 2026-08-14 | Self-hosted web app; live printer telemetry/control; send prints from spoolbook; live camera; cohesive per-printer UI |
| [v0.1.5](docs/releases/v0.1.5.md) | 2026-07-26 | .3mf plate thumbnails on Prints; required-field validation highlighting |
| [v0.1.4](docs/releases/v0.1.4.md) | 2026-07-26 | NumericUpDown rollout; profile field dropdowns; pagination fix |
| [v0.1.3](docs/releases/v0.1.3.md) | 2026-07-25 | Reusable UI controls (StatCard/Pagination/TextBoxWithUnit); profile field editor fixes |
| [v0.1.2](docs/releases/v0.1.2.md) | 2026-07-15 | Printer and Project (`.3mf`) as first-class entities on Print |
| [v0.1.1](docs/releases/v0.1.1.md) | 2026-07-14 | Release packaging fixes (macOS .app bundle, codesigning, per-OS CI smoke tests) |
| [v0.1.0](docs/releases/v0.1.0.md) | 2026-07-14 | Initial Avalonia desktop release |
