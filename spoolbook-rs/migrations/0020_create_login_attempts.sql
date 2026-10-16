-- Failed sign-ins, for per-username throttling (5 within 15 minutes). Rows older than the
-- window are deleted as new ones are recorded; a successful sign-in clears its username's rows.
CREATE TABLE login_attempts (
    id           INTEGER PRIMARY KEY,
    username     TEXT NOT NULL,
    attempted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX login_attempts_username_time ON login_attempts (username, attempted_at);
