ALTER TABLE user_settings
    ADD COLUMN notification_admin_requests BOOLEAN NOT NULL DEFAULT true;