CREATE TABLE projects (
    id                            INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path                     TEXT NOT NULL,
    file_name                     TEXT NOT NULL,
    last_known_write_time_utc     TEXT NOT NULL,
    last_known_file_size_bytes    INTEGER NOT NULL,
    created_at                    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    mesh_hash                     TEXT,
    previous_version_project_id   INTEGER REFERENCES projects(id),
    version_number                INTEGER NOT NULL DEFAULT 1,
    is_current_version             INTEGER NOT NULL DEFAULT 1
);
