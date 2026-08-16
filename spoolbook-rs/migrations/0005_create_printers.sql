CREATE TABLE printers (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT NOT NULL,
    model         TEXT,
    ip_address    TEXT,
    access_code   TEXT,
    serial_number TEXT
);
