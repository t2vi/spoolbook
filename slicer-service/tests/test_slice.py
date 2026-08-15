# Smoke test for slicer-service — run against a live instance (SLICER_SERVICE_URL, default
# http://localhost:8100). Not a unit test: the whole point is proving the real BambuStudio
# binary inside the container can actually slice a real P2S project, not just that the FastAPI
# route logic is wired up. Uses tests/fixtures/sample.3mf, a real unsliced Bambu Studio project
# save (model + project_settings.config, no gcode yet) — an already-sliced fixture wouldn't prove
# slicing actually happened.
import io
import os
import zipfile

import httpx

BASE_URL = os.environ.get("SLICER_SERVICE_URL", "http://localhost:8100")
FIXTURE_PATH = os.path.join(os.path.dirname(__file__), "fixtures", "sample.3mf")


def test_health():
    resp = httpx.get(f"{BASE_URL}/health", timeout=10)
    assert resp.status_code == 200


def test_slice_produces_real_gcode():
    with open(FIXTURE_PATH, "rb") as f:
        resp = httpx.post(
            f"{BASE_URL}/slice",
            files={"project": ("sample.3mf", f, "application/octet-stream")},
            timeout=180,
        )
    assert resp.status_code == 200, resp.text

    archive = zipfile.ZipFile(io.BytesIO(resp.content))
    names = archive.namelist()
    assert "Metadata/plate_1.gcode" in names, f"no sliced gcode in output, entries: {names}"

    gcode = archive.read("Metadata/plate_1.gcode").decode("utf-8", errors="replace")
    assert len(gcode) > 100_000, f"gcode suspiciously small ({len(gcode)} bytes) — did slicing actually run?"

    # The whole reason BambuStudio was chosen over OrcaSlicer: it correctly resolves the P2S's
    # start/end/toolchange gcode macros. Unresolved {template}/[placeholder] syntax in the
    # executable body (i.e. past the CONFIG_BLOCK, which legitimately echoes raw config values
    # as comments) means macro resolution silently failed.
    body = gcode.split("; EXECUTABLE_BLOCK_START", 1)[-1]
    assert "{filament_type" not in body, "unresolved {filament_type...} macro in executed gcode"
    assert "{max_layer_z" not in body, "unresolved {max_layer_z...} macro in executed gcode"
