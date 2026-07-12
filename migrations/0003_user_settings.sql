-- Per-user settings. One row per user who has ever saved their preferences; a
-- user with no row is treated as "all defaults" by the application (see
-- `server::db::settings`), so existing/seeded users keep the default behavior
-- until they opt out.
--
-- Columns are grouped by settings category via a name prefix, mirroring the
-- nested `UserSettings { notifications: NotificationSettings }` model in
-- `server_fns::settings`. Today the only category is email notifications; future
-- categories get their own prefixed columns.
--
-- `notification_emails_enabled` is the notification master switch: when false,
-- no email is sent regardless of the per-category flags. Each remaining boolean
-- toggles one notification category, matching
-- `server_fns::settings::NotificationKind`.

CREATE TABLE user_settings (
    user_id                        TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    notification_emails_enabled    BOOLEAN NOT NULL DEFAULT true,
    notification_new_message       BOOLEAN NOT NULL DEFAULT true,
    notification_case_data         BOOLEAN NOT NULL DEFAULT true,
    notification_note_added        BOOLEAN NOT NULL DEFAULT true,
    notification_evidence_changed  BOOLEAN NOT NULL DEFAULT true,
    notification_assigned          BOOLEAN NOT NULL DEFAULT true
);
