# slicer-service

Thin HTTP wrapper around Bambu Studio's CLI, for spoolbook's "re-slice before send" feature
(re-slicing session, 2026-08-14). Deliberately dumb: one `.3mf` in, one re-sliced `.3mf` out.
All domain logic (translating `PrintProfile` into config, patching that into a copy of the
original project) lives in the main .NET app — see
`Spoolbook.Desktop/Features/Profiles/ProfileConfigPatcher.cs` and
`Spoolbook.Web/Services/ReslicingService.cs`.

Not part of the `Spoolbook.slnx` .NET solution — a separate deployable, meant to run on its own
host (an LXC container, per the intended homelab setup).

## Why Bambu Studio, not OrcaSlicer

Verified live 2026-08-14: OrcaSlicer 2.4.2 (the current stable release — there is no newer one)
fails to slice real spoolbook project files two separate ways — a file-version check (fixable
with `--allow-newer-file`), and, more fundamentally, a parse error in the P2S's own start-gcode
macro syntax (`{filament_type[initial_no_support_filament_id]}`) that OrcaSlicer's older macro
engine doesn't support at all. That's not a flag/config issue, and there's no newer OrcaSlicer
release to move to.

Bambu Studio — the tool that actually produced these project files (exact version match,
`02.07.01.62`) — sliced the same real project cleanly on the first try, no workarounds needed.
It also ships an official Linux AppImage (`github.com/bambulab/BambuStudio` releases,
`ubuntu22.04`/`ubuntu24.04` builds), so it works for the LXC target the same way OrcaSlicer
would have.

## Why a separate process

Bambu Studio's CLI is still a Qt-linked GUI application under the hood — even pure CLI slicing
needs a virtual framebuffer (`Xvfb`) to initialize, and it isn't designed as a headless service.
Isolating it in its own container keeps that mess off the main app's host and makes it trivially
restartable if a slice hangs.

## LXC setup (Debian/Ubuntu base)

```bash
apt update && apt install -y xvfb python3 python3-pip python3-venv

# Get Bambu Studio's Linux AppImage from github.com/bambulab/BambuStudio/releases — pick the
# ubuntu22.04 or ubuntu24.04 build matching (or as close as possible to) whatever Bambu Studio
# version actually produces your project files (mismatches can still work via
# --allow-newer-file, but matching versions avoids the whole class of compatibility issues this
# session ran into with OrcaSlicer).
chmod +x BambuStudio_ubuntu*.AppImage
ln -s /path/to/BambuStudio_ubuntu*.AppImage /usr/local/bin/bambu-studio

python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

## Running

```bash
BAMBUSTUDIO_BIN=bambu-studio uvicorn main:app --host 0.0.0.0 --port 8100
```

Point `Spoolbook.Web`'s `RESLICE_SERVICE_URL` environment variable at this host (e.g.
`http://<lxc-ip>:8100`).

## What's verified vs. not

**Verified** (2026-08-14, against a real macOS Bambu Studio install and real spoolbook project
files, including a real `ProfileConfigPatcher` output sliced end-to-end with the patched values
confirmed present in the resulting gcode): the CLI flags in `main.py`'s `_run_bambustudio`
(`--slice 0 --export-3mf ...`) are correct, and Bambu Studio genuinely slices these real projects
without error.

**Not verified**: the Linux AppImage itself (only the macOS `.app` binary was tested), and
`xvfb-run` specifically (macOS has a real display, so this was never exercised). Same version,
same underlying CLI code as the macOS build, but headless-Linux behavior is the one piece still
worth a manual smoke test once this is on the real LXC:

```bash
xvfb-run -a bambu-studio --slice 0 --export-3mf /tmp/out.3mf /tmp/some-test-project.3mf
```

If that produces a valid `.3mf` with sliced gcode in `Metadata/plate_1.gcode`, the wrapper
works as-is.
