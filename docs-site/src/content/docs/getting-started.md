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

spoolbook is single-user with no account or login. On first launch it seeds a starter filament
catalog and begins syncing fresh data automatically in the background (throttled to once every
24 hours).

## Adding your first spool

1. Go to **Filaments**, find or add the filament type on your roll (brand + material + variant + color).
2. Go to **Spools**, create a Spool against that Filament — this represents the physical roll.
3. Go to **Profiles**, create a Print Profile for it, or import one directly from Bambu Studio.
4. After a print, log it under **Prints** — the exact settings from the Profile version you used
   are locked in permanently, so editing the Profile later (e.g. a seasonal temperature tweak)
   never rewrites what a past print recorded.
