CREATE TABLE spools (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    filament_id   INTEGER NOT NULL REFERENCES filaments(id),
    lot_code      TEXT,
    purchased_at  TEXT,
    opened_at     TEXT,
    emptied_at    TEXT,
    weight_grams  INTEGER,
    diameter_mm   REAL,
    notes         TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
