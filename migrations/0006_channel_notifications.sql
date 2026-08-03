-- Per-user "unread" notifications for case chat.
--
-- One row means: this user has an unread chat message. A notification is created
-- for every user who can read a channel (except the message's author) the moment
-- a message is posted, and the rows for a channel are deleted the moment that
-- user opens it. "Does this user have anything unread?" is therefore just "does a
-- row exist for them?", and "is this channel unread?" is "is there a row for this
-- user on this channel?" — no per-message read-state bookkeeping to reconcile.
--
-- The primary key is (user_id, message_id): a user is notified at most once about
-- any given message, so re-recording is idempotent and clearing a channel is a
-- single delete.
--
-- Every foreign key cascades on delete so a notification can never outlive the
-- user, case, channel, or message it points at. The rows a background task still
-- has to prune are only those that stay unread past the retention window (see
-- `server::db::channel_notifications::purge_expired`, currently one year).
CREATE TABLE channel_notifications (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL REFERENCES case_channels(id) ON DELETE CASCADE,
    case_id    TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, message_id)
);

-- The two hot reads: "everything unread for this user" (the nav badge) and
-- "clear / count this user's unread on one channel" (opening a channel, the
-- per-channel dot).
CREATE INDEX channel_notifications_user_idx ON channel_notifications(user_id);
CREATE INDEX channel_notifications_user_channel_idx ON channel_notifications(user_id, channel_id);

-- Supports the retention sweep that deletes rows left unread past the window.
CREATE INDEX channel_notifications_created_at_idx ON channel_notifications(created_at);
