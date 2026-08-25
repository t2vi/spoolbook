"""Bambu Studio HTTP wrapper — deliberately dumb: one .3mf in, one re-sliced .3mf out.

All domain logic (PrintProfile -> config translation, splicing the patched config back into
a copy of the original .3mf) lives in Spoolbook.Desktop/Spoolbook.Web (C#) — see
ProfileConfigPatcher and ReslicingService. This service only knows how to hand a .3mf to
Bambu Studio's CLI and return whatever comes out. Intended to run in an LXC container (or any
Linux host) with Bambu Studio and Xvfb installed — see README.md for setup.

Bambu Studio, not OrcaSlicer: verified live 2026-08-14 that OrcaSlicer 2.4.2 (the current
stable release) fails to slice real spoolbook project files two ways — a file-version check
(fixable) and, more fundamentally, a parse error in the P2S's own start-gcode macro syntax
that OrcaSlicer's older macro engine doesn't support at all. Bambu Studio — the tool that
actually produced these files, exact version match — sliced the same real project cleanly on
the first try. See docs/adr or the re-slicing session notes for the full comparison.
"""

import asyncio
import os
import shutil
import tempfile
import uuid

from fastapi import FastAPI, HTTPException, UploadFile
from fastapi.responses import Response
from fastapi.staticfiles import StaticFiles

app = FastAPI(title="spoolbook slicer-service")

# Official Linux AppImage: github.com/bambulab/BambuStudio releases (ubuntu22.04/24.04 builds).
BAMBUSTUDIO_BIN = os.environ.get("BAMBUSTUDIO_BIN", "bambu-studio")
SLICE_TIMEOUT_S = int(os.environ.get("SLICE_TIMEOUT_S", "120"))

# The Bambu Studio install this container already carries for slicing ships its own system
# filament preset library at this path (verified empirically against the Dockerfile's own
# extraction: unsquashfs -> /opt/bambustudio/{bin,resources}, resources/profiles/BBL.json +
# resources/profiles/BBL/filament/*.json, same shape spoolbook-rs's bambu_import.rs already
# expects) -- reused as-is rather than pulling a second copy from anywhere else (github.com/t2vi/
# spoolbook/issues/99). Served as plain static files; spoolbook-rs does the inherits-chain
# resolution itself (see bambu_import.rs) since it needs that logic for uploaded presets too.
BAMBU_PROFILES_DIR = os.environ.get("BAMBU_PROFILES_DIR", "/opt/bambustudio/resources/profiles")
if os.path.isdir(BAMBU_PROFILES_DIR):
    app.mount("/profiles", StaticFiles(directory=BAMBU_PROFILES_DIR), name="profiles")


@app.get("/health")
async def health() -> dict:
    return {"ok": True}


@app.post("/slice")
async def slice_project(project: UploadFile) -> Response:
    """Re-slice an uploaded .3mf (with its project_settings.config already patched by the
    caller) and return the resulting .3mf, sliced gcode embedded, unchanged otherwise."""
    if not project.filename or not project.filename.lower().endswith(".3mf"):
        raise HTTPException(status_code=400, detail="Expected a .3mf file")

    work_dir = tempfile.mkdtemp(prefix="spoolbook-slice-")
    input_path = os.path.join(work_dir, f"{uuid.uuid4().hex}.3mf")
    output_path = os.path.join(work_dir, f"{uuid.uuid4().hex}.3mf")

    try:
        with open(input_path, "wb") as f:
            shutil.copyfileobj(project.file, f)

        await _run_bambustudio(input_path, output_path)

        if not os.path.exists(output_path):
            raise HTTPException(status_code=502, detail="Bambu Studio did not produce an output file")

        with open(output_path, "rb") as f:
            sliced_bytes = f.read()

        return Response(content=sliced_bytes, media_type="application/octet-stream")
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


async def _run_bambustudio(input_path: str, output_path: str) -> None:
    # Verified 2026-08-14 against a real Bambu Studio 02.07.01.62 binary (macOS, --help output +
    # actual runs against real spoolbook project files, including a real ProfileConfigPatcher
    # output sliced end-to-end with the patched values confirmed present in the resulting
    # gcode): --slice N (0 = all plates) and --export-3mf <path> are correct as written. No
    # --allow-newer-file needed against a matching/current Bambu Studio version, but it's
    # harmless to keep for whenever the LXC's installed version lags behind whatever produced a
    # given project — a no-op when versions already match.
    #
    # `xvfb-run` wraps it on Linux because Bambu Studio is Qt-linked and needs a virtual
    # framebuffer to initialize even for CLI-only use, no GUI shown. Skipped when xvfb-run isn't
    # on PATH — macOS (and Linux with a real display) has an actual display to render to, so
    # there's nothing to fake. Not tested against the real Linux AppImage or under xvfb-run
    # specifically — only the macOS .app binary with a real display was available while verifying
    # this. Same version, same underlying CLI code, but treat headless-Linux behavior as the one
    # still-unverified piece.
    bambu_args = [
        BAMBUSTUDIO_BIN,
        "--allow-newer-file",
        "--slice",
        "0",
        "--export-3mf",
        output_path,
        input_path,
    ]
    cmd = ["xvfb-run", "-a", *bambu_args] if shutil.which("xvfb-run") else bambu_args

    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=SLICE_TIMEOUT_S)
    except asyncio.TimeoutError:
        proc.kill()
        await proc.wait()
        raise HTTPException(status_code=504, detail=f"Bambu Studio timed out after {SLICE_TIMEOUT_S}s")

    if proc.returncode != 0:
        detail = stderr.decode(errors="replace").strip() or stdout.decode(errors="replace").strip()
        raise HTTPException(status_code=502, detail=f"Bambu Studio exited {proc.returncode}: {detail[-2000:]}")
