---
title: Getting Started
order: 1
---

## Installing

spoolbook is a self-hosted web app — reachable from a phone or any device on your LAN, not just
the machine it's running on. The one-click install runs it via Docker Compose:

```
curl -fsSL https://raw.githubusercontent.com/t2vi/spoolbook/main/install.sh | bash
```

This pulls the pre-built `spoolbook` app image plus a `spoolbook-slicer` service (BambuStudio,
headless — used for re-slicing before print) from GHCR, prompts for an admin password, and starts
both. Works anywhere Docker runs: bare metal, a VM, or a Proxmox LXC (enable nesting first if
running inside an unprivileged LXC). Requires Docker + the Compose plugin.

Once it's up, it's at `http://<host-ip>:5070`.

## First run

spoolbook is single-user. The admin password you set during install gates anything that changes
data (editing, deleting, sending a print) — just browsing stays open to anyone on your LAN.
Optionally link a Google account under **Settings → Account** for one-click sign-in afterward —
your password still works either way. On first launch it seeds a starter filament catalog and
begins syncing fresh data automatically in the background (throttled to once every 24 hours).

## Building from source instead

```
cd spoolbook-web-svelte && npm run build   # frontend, needed once before first run
cd ../spoolbook-rs && cargo run --release
```

Set `SPOOLBOOK_ADMIN_PASSWORD` first. Useful for development or if you'd rather not use Docker;
see the [repo README](https://github.com/t2vi/spoolbook) for the full dev workflow.

## Printer control and live camera

Add a printer under **Settings → Printers** with its IP address and access code (found on the
printer's own network settings screen) to connect. Once connected, the **Printers** page shows a
live card per printer: current status and temperatures, full AMS tray contents, pause/resume/stop
controls, and a live camera view (opens in a popup window). The **Print** button on a card lets
you send a `.3mf` project straight to that printer — pick the plate and AMS slot, and spoolbook
creates the print-history entry automatically, filling in its outcome once the printer reports
the job finished.

## Adding your first spool

1. Go to **Filaments → List**, find or add the filament type on your roll (brand + material +
   variant + color).
2. Go to **Filaments → Spools**, create a Spool against that Filament — this represents the
   physical roll.
3. Go to **Print workflow → Profiles**, create a Print Profile for it — start from a Bambu Studio
   default preset, import a sliced `.3mf` or your own saved Bambu Studio `.json` preset, or fill
   it in by hand.
4. Send the print (see above) — spoolbook creates the Print record automatically and locks in the
   exact settings from the Profile version you used, so editing the Profile later (e.g. a seasonal
   temperature tweak) never rewrites what a past print recorded. **Print workflow → Prints** shows
   the full history; each entry is read-only, including the plate thumbnail spoolbook read out of
   the project at send time.
