CREATE TABLE print_hourly_weather (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    print_id     INTEGER NOT NULL REFERENCES prints(id) ON DELETE CASCADE,
    hour         TEXT NOT NULL,
    temp_c       REAL,
    humidity_pct REAL,
    UNIQUE (print_id, hour)
);
