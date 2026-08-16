CREATE TABLE printer_jobs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    printer_id      INTEGER NOT NULL REFERENCES printers(id) ON DELETE CASCADE,
    external_job_id TEXT NOT NULL,
    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    print_id        INTEGER REFERENCES prints(id) ON DELETE SET NULL
);

CREATE TABLE printer_readings (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    printer_job_id INTEGER NOT NULL REFERENCES printer_jobs(id) ON DELETE CASCADE,
    recorded_at    TEXT NOT NULL,
    nozzle_temp_c  REAL,
    bed_temp_c     REAL,
    chamber_temp_c REAL,
    ams_slot       TEXT,
    progress_pct   INTEGER
);

CREATE INDEX idx_printer_jobs_printer_id ON printer_jobs(printer_id);
CREATE INDEX idx_printer_jobs_print_id ON printer_jobs(print_id);
CREATE INDEX idx_printer_readings_printer_job_id ON printer_readings(printer_job_id);
