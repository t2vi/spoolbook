ALTER TABLE printer_readings ADD COLUMN ams_humidity_pct INTEGER;
ALTER TABLE printer_readings ADD COLUMN layer_num INTEGER;
ALTER TABLE printer_readings ADD COLUMN total_layer_num INTEGER;
ALTER TABLE prints ADD COLUMN telemetry_json TEXT;
