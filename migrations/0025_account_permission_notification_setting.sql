-- Generalize the former case-assignment preference without changing anyone's choice.
ALTER TABLE user_settings
    RENAME COLUMN notification_assigned TO notification_account_permissions_changed;
