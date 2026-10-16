-- Export bundles this install has produced or imported, so the same bundle is never imported
-- twice (and an install never imports its own export).
CREATE TABLE data_bundles (
    bundle_id   TEXT PRIMARY KEY,
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
