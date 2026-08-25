ALTER TABLE users ADD COLUMN google_sub TEXT;
CREATE UNIQUE INDEX idx_users_google_sub ON users(google_sub);

ALTER TABLE app_settings ADD COLUMN google_client_id TEXT;
ALTER TABLE app_settings ADD COLUMN google_client_secret TEXT;
ALTER TABLE app_settings ADD COLUMN google_redirect_uri TEXT;
