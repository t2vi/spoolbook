-- Whether a person set this color's hex (Colors screen); the catalog sync never overwrites those
-- (007/FR-027). A hex list already stored was put there deliberately, so it counts as theirs.
ALTER TABLE filament_colors ADD COLUMN hex_set_by_user INTEGER NOT NULL DEFAULT 0;
UPDATE filament_colors SET hex_set_by_user = 1 WHERE hex LIKE '%,%';
