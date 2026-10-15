-- Desktop-era settings (local Bambu Studio preset/profile folders); nothing reads or writes them
-- since profiles stopped coming from local files.
ALTER TABLE app_settings DROP COLUMN bambu_user_presets_dir;
ALTER TABLE app_settings DROP COLUMN bambu_system_profiles_dir;
