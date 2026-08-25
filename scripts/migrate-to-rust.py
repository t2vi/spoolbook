#!/usr/bin/env python3
"""One-time migration: .NET/EF Core spoolbook.db -> spoolbook-rs's own SQLite schema.

Per docs/adr/0026: Rust develops against its own DB; a single one-time migration
copies real data across at cutover, then .NET is deleted. Source DB is opened
read-only (immutable=1) -- this script never writes to it.

Usage:
    python3 scripts/migrate-to-rust.py <source_spoolbook.db> <target_new.db>

The target file must not already exist -- this always creates a fresh DB from
spoolbook-rs/migrations/*.sql, then populates it. Re-run by deleting the target
and starting over; never appends to an existing target.
"""
import os
import sqlite3
import subprocess
import sys
import pathlib

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
MIGRATIONS_DIR = REPO_ROOT / "spoolbook-rs" / "migrations"

# Enum int -> Rust string mapping, taken verbatim from the C# enum declarations:
#   Spoolbook.Desktop/Features/Prints/Print.cs (PrintStatus, AmbientSource)
#   Spoolbook.Desktop/Features/Prints/FailureMode.cs (FailureMode)
#   Spoolbook.Desktop/Features/Profiles/PrintProfile.cs (ProfileSource, SlicerType)
PRINT_STATUS = ["Success", "Failed", "Partial", "InProgress"]
AMBIENT_SOURCE = ["WeatherApi", "Sensor", "Manual"]
FAILURE_MODE = ["Stringing", "LayerAdhesion", "Warping", "UnderExtrusion", "OverExtrusion", "LayerShift", "Clog", "Other"]
PROFILE_SOURCE = ["Manual", "SlicerImport"]
SLICER_TYPE = ["PrusaSlicer", "OrcaSlicer", "BambuStudio"]


def enum_str(values, i):
    return None if i is None else values[i]


def to_real(v):
    """EF Core stores decimal? as TEXT; rust stores REAL. Parse, passing None/'' through as NULL."""
    if v is None or v == "":
        return None
    return float(v)


# Per-table column plan: (source_column, target_column, converter_or_None).
# converter is applied to the source value; None means direct passthrough.
# FK columns are handled separately via id_maps, not listed here.
FILAMENTS = [("Id", "id", None), ("Brand", "brand", None), ("Material", "material", None),
             ("Variant", "variant", None), ("Color", "color", None)]

FILAMENT_COLORS = [("Id", "id", None), ("Name", "name", None), ("Hex", "hex", None)]

SPOOLS = [("Id", "id", None), ("LotCode", "lot_code", None), ("PurchasedAt", "purchased_at", None),
          ("OpenedAt", "opened_at", None), ("EmptiedAt", "emptied_at", None),
          ("WeightGrams", "weight_grams", None), ("DiameterMm", "diameter_mm", to_real),
          ("Notes", "notes", None), ("CreatedAt", "created_at", None)]

PRINTERS = [("Id", "id", None), ("Name", "name", None), ("Model", "model", None),
            ("IpAddress", "ip_address", None), ("AccessCode", "access_code", None),
            ("SerialNumber", "serial_number", None)]

PROJECTS = [("Id", "id", None), ("FilePath", "file_path", None), ("FileName", "file_name", None),
            ("LastKnownWriteTimeUtc", "last_known_write_time_utc", None),
            ("LastKnownFileSizeBytes", "last_known_file_size_bytes", None),
            ("CreatedAt", "created_at", None), ("MeshHash", "mesh_hash", None),
            ("VersionNumber", "version_number", None), ("IsCurrentVersion", "is_current_version", None)]
# PreviousVersionProjectId handled in a second pass (self-referencing FK).

PRINT_PROFILES = [
    ("Id", "id", None), ("Name", "name", None), ("PrintSpeedMmS", "print_speed_mm_s", None),
    ("NozzleTempC", "nozzle_temp_c", None), ("NozzleTempInitialC", "nozzle_temp_initial_c", None),
    ("NozzleTempRangeHighC", "nozzle_temp_range_high_c", None), ("NozzleTempRangeLowC", "nozzle_temp_range_low_c", None),
    ("CoolPlateTempC", "cool_plate_temp_c", None), ("CoolPlateTempInitialC", "cool_plate_temp_initial_c", None),
    ("HotPlateTempC", "hot_plate_temp_c", None), ("HotPlateTempInitialC", "hot_plate_temp_initial_c", None),
    ("TexturedPlateTempC", "textured_plate_temp_c", None), ("TexturedPlateTempInitialC", "textured_plate_temp_initial_c", None),
    ("EngPlateTempC", "eng_plate_temp_c", None), ("EngPlateTempInitialC", "eng_plate_temp_initial_c", None),
    ("SupertackPlateTempC", "supertack_plate_temp_c", None), ("SupertackPlateTempInitialC", "supertack_plate_temp_initial_c", None),
    ("FanMinSpeedPct", "fan_min_speed_pct", None), ("FanMaxSpeedPct", "fan_max_speed_pct", None),
    ("AdditionalCoolingFanSpeedPct", "additional_cooling_fan_speed_pct", None),
    ("CloseFanFirstXLayers", "close_fan_first_x_layers", None),
    ("CompletePrintExhaustFanSpeedPct", "complete_print_exhaust_fan_speed_pct", None),
    ("DuringPrintExhaustFanSpeedPct", "during_print_exhaust_fan_speed_pct", None),
    ("ChamberTemperatureC", "chamber_temperature_c", None),
    ("CoolingPerimeterTransitionDistanceMm", "cooling_perimeter_transition_distance_mm", to_real),
    ("CoolingSlowdownLogic", "cooling_slowdown_logic", None),
    ("EnableOverhangBridgeFan", "enable_overhang_bridge_fan", None),
    ("FanCoolingLayerTimeS", "fan_cooling_layer_time_s", None),
    ("FirstXLayerFanSpeedPct", "first_x_layer_fan_speed_pct", None),
    ("FullFanSpeedLayer", "full_fan_speed_layer", None),
    ("NoSlowDownForCoolingOnOutwalls", "no_slow_down_for_cooling_on_outwalls", None),
    ("OverhangFanSpeedPct", "overhang_fan_speed_pct", None),
    ("OverhangFanThreshold", "overhang_fan_threshold", None),
    ("OverhangThresholdParticipatingCooling", "overhang_threshold_participating_cooling", None),
    ("OverrideProcessOverhangSpeed", "override_process_overhang_speed", None),
    ("PreStartFanTimeS", "pre_start_fan_time_s", None),
    ("ReduceFanStopStartFreq", "reduce_fan_stop_start_freq", None),
    ("SlowDownForLayerCooling", "slow_down_for_layer_cooling", None),
    ("SlowDownLayerTimeS", "slow_down_layer_time_s", None),
    ("SlowDownMinSpeedMmS", "slow_down_min_speed_mm_s", None),
    ("ActivateAirFiltration", "activate_air_filtration", None),
    ("RetractionMm", "retraction_mm", to_real), ("RetractionSpeedMmS", "retraction_speed_mm_s", to_real),
    ("DeretractionSpeedMmS", "deretraction_speed_mm_s", to_real),
    ("RetractionMinimumTravelMm", "retraction_minimum_travel_mm", to_real),
    ("RetractBeforeWipe", "retract_before_wipe", None),
    ("RetractRestartExtraMm", "retract_restart_extra_mm", to_real),
    ("RetractWhenChangingLayer", "retract_when_changing_layer", None),
    ("RetractionDistancesWhenCutMm", "retraction_distances_when_cut_mm", to_real),
    ("RetractLengthNcMm", "retract_length_nc_mm", to_real),
    ("LongRetractionsWhenCut", "long_retractions_when_cut", None),
    ("LongRetractionsWhenEc", "long_retractions_when_ec", None),
    ("RetractionDistancesWhenEcMm", "retraction_distances_when_ec_mm", to_real),
    ("WipeEnabled", "wipe_enabled", None), ("WipeDistanceMm", "wipe_distance_mm", to_real),
    ("ZHopMm", "z_hop_mm", to_real), ("ZHopType", "z_hop_type", None),
    ("ChangeLengthMm", "change_length_mm", to_real), ("ChangeLengthNcMm", "change_length_nc_mm", to_real),
    ("CoolingBeforeTowerS", "cooling_before_tower_s", None),
    ("MinimalPurgeOnWipeTowerMm3", "minimal_purge_on_wipe_tower_mm3", to_real),
    ("PrimeVolumeMm3", "prime_volume_mm3", to_real), ("PrimeVolumeNcMm3", "prime_volume_nc_mm3", to_real),
    ("RammingTravelTimeS", "ramming_travel_time_s", to_real), ("RammingTravelTimeNcS", "ramming_travel_time_nc_s", to_real),
    ("RammingVolumetricSpeedMm3S", "ramming_volumetric_speed_mm3_s", to_real),
    ("RammingVolumetricSpeedNcMm3S", "ramming_volumetric_speed_nc_mm3_s", to_real),
    ("TowerInterfacePreExtrusionDistMm", "tower_interface_pre_extrusion_dist_mm", to_real),
    ("TowerInterfacePreExtrusionLengthMm", "tower_interface_pre_extrusion_length_mm", to_real),
    ("TowerInterfacePrintTempC", "tower_interface_print_temp_c", None),
    ("TowerInterfacePurgeVolumeMm3", "tower_interface_purge_volume_mm3", to_real),
    ("TowerIroningAreaMm2", "tower_ironing_area_mm2", to_real),
    ("FlushTempC", "flush_temp_c", None), ("FlushVolumetricSpeedMm3S", "flush_volumetric_speed_mm3_s", to_real),
    ("AdaptiveVolumetricSpeed", "adaptive_volumetric_speed", None),
    ("MaxVolumetricSpeedMm3S", "max_volumetric_speed_mm3_s", to_real),
    ("BridgeSpeedMmS", "bridge_speed_mm_s", to_real), ("EnableOverhangSpeed", "enable_overhang_speed", None),
    ("Overhang14SpeedMmS", "overhang_14_speed_mm_s", to_real), ("Overhang24SpeedMmS", "overhang_24_speed_mm_s", to_real),
    ("Overhang34SpeedMmS", "overhang_34_speed_mm_s", to_real), ("Overhang44SpeedMmS", "overhang_44_speed_mm_s", to_real),
    ("OverhangTotallySpeedMmS", "overhang_totally_speed_mm_s", to_real),
    ("CircleCompensationSpeedMmS", "circle_compensation_speed_mm_s", to_real),
    ("VelocityAdaptationFactor", "velocity_adaptation_factor", to_real),
    ("VolumetricSpeedCoefficients", "volumetric_speed_coefficients", None),
    ("DensityGCm3", "density_g_cm3", to_real), ("DiameterMm", "diameter_mm", to_real),
    ("DiameterLimitMm", "diameter_limit_mm", to_real), ("ShrinkPct", "shrink_pct", None),
    ("Soluble", "soluble", None), ("IsSupport", "is_support", None), ("Printable", "printable", None),
    ("AdhesivenessCategory", "adhesiveness_category", None),
    ("ImpactStrengthZ", "impact_strength_z", to_real), ("CostPerKg", "cost_per_kg", to_real),
    ("FlowRatio", "flow_ratio", to_real), ("ExtruderVariant", "extruder_variant", None),
    ("SlicerNotes", "slicer_notes", None), ("RequiredNozzleHrc", "required_nozzle_hrc", None),
    ("EnablePressureAdvance", "enable_pressure_advance", None), ("PressureAdvance", "pressure_advance", to_real),
    ("DryingAmsLimitations", "drying_ams_limitations", None),
    ("DryingAmsHeatDistortionTempC", "drying_ams_heat_distortion_temp_c", None),
    ("DryingAmsTempC", "drying_ams_temp_c", None), ("DryingAmsTimeH", "drying_ams_time_h", to_real),
    ("DryingChamberBedTempC", "drying_chamber_bed_temp_c", None), ("DryingChamberTimeH", "drying_chamber_time_h", to_real),
    ("DryingCoolingTempC", "drying_cooling_temp_c", None), ("DryingSofteningTempC", "drying_softening_temp_c", None),
    ("SofteningTempC", "softening_temp_c", to_real),
    ("ScarfSeamType", "scarf_seam_type", None), ("ScarfGapPct", "scarf_gap_pct", None),
    ("ScarfHeightPct", "scarf_height_pct", None), ("ScarfLengthMm", "scarf_length_mm", to_real),
    ("HoleCoef1", "hole_coef_1", to_real), ("HoleCoef2", "hole_coef_2", to_real), ("HoleCoef3", "hole_coef_3", to_real),
    ("HoleLimitMax", "hole_limit_max", to_real), ("HoleLimitMin", "hole_limit_min", to_real),
    ("CounterCoef1", "counter_coef_1", to_real), ("CounterCoef2", "counter_coef_2", to_real),
    ("CounterCoef3", "counter_coef_3", to_real),
    ("CounterLimitMax", "counter_limit_max", to_real), ("CounterLimitMin", "counter_limit_min", to_real),
    ("StartGcode", "start_gcode", None), ("EndGcode", "end_gcode", None),
    ("DefaultColourHex", "default_colour_hex", None),
    ("Source", "source", lambda v: enum_str(PROFILE_SOURCE, v)),
    ("SourceSlicer", "source_slicer", lambda v: enum_str(SLICER_TYPE, v)),
    ("RawSettingsJson", "raw_settings_json", None), ("SourcePresetPath", "source_preset_path", None),
    ("VersionNumber", "version_number", None), ("VersionName", "version_name", None),
    ("IsCurrentVersion", "is_current_version", None), ("Notes", "notes", None), ("CreatedAt", "created_at", None),
]
# FilamentId, SpoolId handled via id_maps, not listed above.

PRINTS = [
    ("Id", "id", None), ("ProjectPlaterId", "project_plater_id", None),
    ("StartedAt", "started_at", None), ("EndedAt", "ended_at", None),
    ("Status", "status", lambda v: enum_str(PRINT_STATUS, v)),
    ("Notes", "notes", None), ("AmbientTempC", "ambient_temp_c", to_real),
    ("AmbientHumidityPct", "ambient_humidity_pct", to_real),
    ("AmbientSource", "ambient_source", lambda v: enum_str(AMBIENT_SOURCE, v)),
    ("AmsHumidityPct", "ams_humidity_pct", None), ("ActualRoomTempC", "actual_room_temp_c", to_real),
    ("CleanBuildPlate", "clean_build_plate", None), ("CreatedAt", "created_at", None),
]
# ProfileId, SpoolId, PrinterId, ProjectId handled via id_maps.

PRINT_FAILURE_MODES = [("Id", "id", None), ("Mode", "mode", lambda v: enum_str(FAILURE_MODE, v))]
# PrintId handled via id_map.

APP_SETTINGS = [("BambuUserPresetsDir", "bambu_user_presets_dir", None),
                ("BambuSystemProfilesDir", "bambu_system_profiles_dir", None),
                ("LastFilamentSyncAt", "last_filament_sync_at", None),
                ("AdditionalFilamentSourceUrls", "additional_filament_source_urls", None)]


def create_target_schema(target_path: str):
    # Delegates to sqlx-cli rather than executing the .sql files directly, so the resulting DB
    # has a correct _sqlx_migrations bookkeeping table -- otherwise spoolbook-rs's own startup
    # migrate!().run() sees an unmigrated DB, tries to reapply migration 0001, and panics with
    # "table filaments already exists".
    env = {**os.environ, "DATABASE_URL": f"sqlite://{target_path}"}
    subprocess.run(["sqlx", "database", "create"], env=env, check=True)
    subprocess.run(["sqlx", "migrate", "run", "--source", str(MIGRATIONS_DIR)], env=env, check=True)


def copy_table(source, target, table, plan, id_map_cols=None, id_maps=None, extra_where=""):
    """Copies one table using an explicit (src_col, dst_col, converter) plan.
    id_map_cols: {source_fk_col: (dst_fk_col, id_map_dict, nullable)} for FK remap.
    Returns {old_id: new_id}.
    """
    id_map_cols = id_map_cols or {}
    src_cols = [p[0] for p in plan] + list(id_map_cols.keys())
    quoted_cols = ", ".join('"' + c + '"' for c in src_cols)
    rows = source.execute(f'SELECT {quoted_cols} FROM "{table}"{extra_where}').fetchall()

    new_ids = {}
    for row in rows:
        row = dict(zip(src_cols, row))
        values = {}
        for src_col, dst_col, conv in plan:
            v = row[src_col]
            values[dst_col] = conv(v) if conv else v
        skip = False
        for src_col, (dst_col, id_map, nullable) in id_map_cols.items():
            old_fk = row[src_col]
            if old_fk is None:
                if not nullable:
                    skip = True
                    break
                values[dst_col] = None
            else:
                if old_fk not in id_map:
                    skip = True
                    break
                values[dst_col] = id_map[old_fk]
        if skip:
            print(f"  SKIP {table} id={row.get('Id')}: unresolved FK")
            continue

        old_id = row.get("Id")
        cols = list(values.keys())
        placeholders = ", ".join("?" for _ in cols)
        cur = target.execute(
            f'INSERT INTO {_snake_table(table)} ({", ".join(cols)}) VALUES ({placeholders})',
            [values[c] for c in cols],
        )
        if old_id is not None:
            new_ids[old_id] = cur.lastrowid
    return new_ids


def _snake_table(name):
    return {
        "Filaments": "filaments", "FilamentColors": "filament_colors", "Spools": "spools",
        "PrintProfiles": "print_profiles", "Printers": "printers", "Projects": "projects",
        "Prints": "prints", "PrintFailureModes": "print_failure_modes",
    }[name]


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(1)
    source_path, target_path = sys.argv[1], sys.argv[2]

    if pathlib.Path(target_path).exists():
        print(f"Target {target_path} already exists — refusing to overwrite. Delete it first if you want a redo.")
        sys.exit(1)

    create_target_schema(target_path)

    source = sqlite3.connect(f"file:{source_path}?mode=ro", uri=True)
    target = sqlite3.connect(target_path)
    target.execute("PRAGMA foreign_keys = OFF")  # re-enabled after data load, checked explicitly below

    filament_ids = copy_table(source, target, "Filaments", FILAMENTS)
    copy_table(source, target, "FilamentColors", FILAMENT_COLORS)
    spool_ids = copy_table(source, target, "Spools", SPOOLS,
                            id_map_cols={"FilamentId": ("filament_id", filament_ids, False)})
    profile_ids = copy_table(source, target, "PrintProfiles", PRINT_PROFILES, id_map_cols={
        "FilamentId": ("filament_id", filament_ids, False),
        "SpoolId": ("spool_id", spool_ids, True),
    })
    printer_ids = copy_table(source, target, "Printers", PRINTERS)
    project_ids = copy_table(source, target, "Projects", PROJECTS)
    # Second pass: self-referencing PreviousVersionProjectId, now that every project has a new id.
    for old_id, new_id in project_ids.items():
        prev = source.execute('SELECT "PreviousVersionProjectId" FROM "Projects" WHERE "Id" = ?', (old_id,)).fetchone()[0]
        if prev is not None and prev in project_ids:
            target.execute("UPDATE projects SET previous_version_project_id = ? WHERE id = ?", (project_ids[prev], new_id))

    print_ids = copy_table(source, target, "Prints", PRINTS, id_map_cols={
        "ProfileId": ("profile_id", profile_ids, False),
        "SpoolId": ("spool_id", spool_ids, False),
        "PrinterId": ("printer_id", printer_ids, False),
        "ProjectId": ("project_id", project_ids, True),
    })
    copy_table(source, target, "PrintFailureModes", PRINT_FAILURE_MODES,
               id_map_cols={"PrintId": ("print_id", print_ids, False)})

    settings_row = source.execute(
        'SELECT "BambuUserPresetsDir", "BambuSystemProfilesDir", "LastFilamentSyncAt", "AdditionalFilamentSourceUrls" '
        'FROM "AppSettings" WHERE "Id" = 1'
    ).fetchone()
    if settings_row:
        target.execute(
            "UPDATE app_settings SET bambu_user_presets_dir = ?, bambu_system_profiles_dir = ?, "
            "last_filament_sync_at = ?, additional_filament_source_urls = ? WHERE id = 1",
            settings_row,
        )

    target.commit()

    print("\nSkipped tables (ephemeral, rebuilt live from MQTT): PrinterJobs, PrinterReadings")

    # One runnable check: row-count parity per table + a real FK-integrity pass, not just a hope.
    print("\n=== row counts (source -> target) ===")
    ok = True
    for src_table, tgt_table in [
        ("Filaments", "filaments"), ("FilamentColors", "filament_colors"), ("Spools", "spools"),
        ("PrintProfiles", "print_profiles"), ("Printers", "printers"), ("Projects", "projects"),
        ("Prints", "prints"), ("PrintFailureModes", "print_failure_modes"),
    ]:
        src_n = source.execute(f'SELECT COUNT(*) FROM "{src_table}"').fetchone()[0]
        tgt_n = target.execute(f"SELECT COUNT(*) FROM {tgt_table}").fetchone()[0]
        flag = "OK" if src_n == tgt_n else "MISMATCH"
        if src_n != tgt_n:
            ok = False
        print(f"  {src_table:20s} {src_n:6d} -> {tgt_n:6d}  {flag}")

    target.execute("PRAGMA foreign_keys = ON")
    fk_errors = target.execute("PRAGMA foreign_key_check").fetchall()
    if fk_errors:
        ok = False
        print(f"\nFK CHECK FAILED: {len(fk_errors)} violation(s)")
        for e in fk_errors[:10]:
            print(f"  {e}")
    else:
        print("\nFK check: OK, no violations")

    print("\nRESULT:", "PASS" if ok else "FAIL — do not use this DB for real")
    source.close()
    target.close()


if __name__ == "__main__":
    main()
