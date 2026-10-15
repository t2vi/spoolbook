-- Chains PrintProfile versions the same way projects chain (previous_version_project_id).
ALTER TABLE print_profiles ADD COLUMN previous_version_profile_id INTEGER REFERENCES print_profiles(id);
