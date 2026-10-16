-- Room temperature near the printer, entered by hand when annotating a finished Print's outcome
-- (the old actual_room_temp_c was renamed to chamber_temp_c in 0013, which is telemetry).
ALTER TABLE prints ADD COLUMN actual_room_temp_c REAL;
