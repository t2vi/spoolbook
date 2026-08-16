CREATE TABLE app_settings (
    id                              INTEGER PRIMARY KEY,
    bambu_user_presets_dir          TEXT,
    bambu_system_profiles_dir       TEXT,
    last_filament_sync_at           TEXT,
    additional_filament_source_urls TEXT
);
