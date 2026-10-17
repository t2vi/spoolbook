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

spoolbook is single-user. You sign in with the admin account you set during install; until then
it shows only the sign-in page — nothing in your notebook is visible to others on your LAN.
Optionally link a Google account under **Settings → Account** for one-click sign-in afterward —
your password still works either way. After 5 wrong passwords in 15 minutes, sign-in for that
username pauses for 15 minutes; changing your password signs out every other device. On first
launch it seeds a starter filament catalog and begins syncing fresh data automatically in the
background (throttled to once every 24 hours).

The moon/sun button at the top right switches between light and dark. Until you pick one it
follows your system setting; your choice is remembered in that browser.

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

## Migrating between installs

**Settings → Data** lets you move data from one spoolbook install to another without shell or
Docker access to either host — moving to new hardware, for example. **Export** downloads a zip of
everything (filaments, spools, print profiles, printers, prints, and their uploaded `.3mf` project
files). On the target install, pick that file and **Preview import** to see how many rows each
table will add before anything is written, then **Confirm import** to merge it in. Filaments,
printers, and filament colors that already exist (matched by name/type) aren't duplicated;
everything else is added fresh — so importing combines two installs' data rather than overwriting
one with the other. Each export can be imported once: importing the same file again — or an
install's own export back into itself — is refused with "already imported", so nothing gets
duplicated by accident. (Exports made before v1.6 don't carry that marker, so check the preview
counts before confirming one of those twice.)

## Adding your first spool

1. Go to **Filaments → List**, find or add the filament type on your roll (brand + material +
   variant + color). **Filaments → Colors** holds each color's hex code — give a multi-color
   filament several (`#1A1A1A, #D4AF37`) and its swatch shows them all. Colors from the catalog
   arrive with their hex (several for a multi-color filament) and the catalog sync keeps them up
   to date — except one you've edited, which stays yours. Adding or editing anything opens a
   form over the list you're on; closing it with unsaved changes asks first.
2. Go to **Filaments → Spools**, create a Spool against that Filament — this represents the
   physical roll.
3. Go to **Print workflow → Profiles**, create a Print Profile for it — start from a Bambu Studio
   default preset, import a sliced `.3mf` or your own saved Bambu Studio `.json` preset, or fill
   it in by hand. The **Spool** picker makes it generic (any roll of that filament) or specific to
   one spool.
4. Send the print (see above) — spoolbook creates the Print record automatically and locks in the
   exact settings from the Profile version you used. That version is then locked: to tweak it
   later (e.g. a seasonal temperature change), open it and use **Save as new version**, so a past
   print always shows what it was really printed with. A profile's **Versions** panel lists them
   all — name one ("Winter 2026") and use **Use this version** to switch which one you print with.
   **Print workflow → Prints** shows the full history; each entry fills in automatically once the
   print ends — a photo of the finished bed, outdoor weather for the print window, and (P2S/AMS
   hardware that reports it) chamber temperature, AMS humidity, and layer progress charted over
   the print. Use **Edit outcome** on a finished print to record how it came out: result, failure
   modes, notes, room temperature and whether the plate was clean.
