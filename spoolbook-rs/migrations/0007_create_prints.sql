CREATE TABLE prints (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id             INTEGER NOT NULL REFERENCES print_profiles(id),
    spool_id               INTEGER NOT NULL REFERENCES spools(id),
    printer_id             INTEGER NOT NULL REFERENCES printers(id),
    project_id             INTEGER REFERENCES projects(id),
    project_plater_id      TEXT,
    started_at             TEXT NOT NULL,
    ended_at               TEXT,
    status                 TEXT NOT NULL,
    notes                  TEXT,
    ambient_temp_c         REAL,
    ambient_humidity_pct   REAL,
    ambient_source         TEXT,
    ams_humidity_pct       INTEGER,
    actual_room_temp_c     REAL,
    clean_build_plate      INTEGER,
    created_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE print_failure_modes (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    print_id INTEGER NOT NULL REFERENCES prints(id),
    mode     TEXT NOT NULL
);
