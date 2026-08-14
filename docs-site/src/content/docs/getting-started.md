---
title: Getting Started
order: 1
---

## Installing

Download the build for your OS from the [Downloads](/spoolbook/downloads) page.

- **macOS**: unzip, right-click `Spoolbook.app` → Open (first launch only — it's ad-hoc signed, not notarized, so Gatekeeper will warn once).
- **Windows**: unzip, run `Spoolbook.Desktop.exe`.
- **Linux**: unzip, `chmod +x Spoolbook.Desktop && ./Spoolbook.Desktop`.

## First run

spoolbook is single-user with no account or login on the desktop app. On first launch it seeds a
starter filament catalog and begins syncing fresh data automatically in the background (throttled
to once every 24 hours).

## Running the web app instead (self-hosted)

spoolbook can also run as a self-hosted web app (`Spoolbook.Web`) — reachable from a phone or any
device on your LAN, not just the machine it's running on, and needed for live printer status,
control, and camera (see below). It uses the same SQLite database as the desktop app, so either
one can be used, including on the same machine at the same time.

Build and run from source (there's no packaged download for this yet):

```
dotnet run --project Spoolbook.Web -c Release
```

Set the `SPOOLBOOK_ADMIN_PASSWORD` environment variable first — the web app gates anything that
changes data (editing, deleting, sending a print) behind that shared password; just browsing
stays open to anyone on your LAN.

## Printer control and live camera

Add a printer under **Settings → Printers** with its IP address and access code (found on the
printer's own network settings screen) to connect. Once connected, the **Printers** page shows a
live card per printer: current status and temperatures, full AMS tray contents, pause/resume/stop
controls, and a live camera view (opens in a popup window). The **Print** button on a card lets
you send a `.3mf` project straight to that printer — pick the plate and AMS slot, and spoolbook
creates the print-history entry automatically, filling in its outcome once the printer reports
the job finished.

## Adding your first spool

1. Go to **Filaments**, find or add the filament type on your roll (brand + material + variant + color).
2. Go to **Spools**, create a Spool against that Filament — this represents the physical roll.
3. Go to **Profiles**, create a Print Profile for it, or import one directly from Bambu Studio.
4. After a print, log it under **Prints** — the exact settings from the Profile version you used
   are locked in permanently, so editing the Profile later (e.g. a seasonal temperature tweak)
   never rewrites what a past print recorded. Optionally attach the `.3mf` project file you sliced
   — spoolbook reads the plate thumbnail straight out of it, and if the file has more than one
   build-plate layout you can pick which one this Print corresponds to.
