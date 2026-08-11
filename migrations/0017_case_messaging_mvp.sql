-- Secure case-messaging MVP hardening.
--
-- Adds channel archival, audience-aware audit metadata, durable first-read
-- evidence, send-throttle state, and relational checks that keep message history
-- attached to the channel/case it was written under.

-- ---------------------------------------------------------------------------
-- Channel lifecycle
-- ---------------------------------------------------------------------------
ALTER TABLE case_channels
    ADD COLUMN state       TEXT NOT NULL DEFAULT 'active',
    ADD COLUMN is_permanent BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN archived_at TIMESTAMPTZ,
    ADD COLUMN archived_by TEXT NOT NULL DEFAULT '';

UPDATE case_channels
SET is_permanent = true
WHERE kind = 'volunteer_only';

ALTER TABLE case_channels
    ADD CONSTRAINT case_channels_state_check CHECK (state IN ('active', 'archived')),
    ADD CONSTRAINT case_channels_kind_state_check CHECK (kind IN ('standard', 'volunteer_only')),
    ADD CONSTRAINT case_channels_permanent_volunteer_check CHECK (kind <> 'volunteer_only' OR is_permanent),
    ADD CONSTRAINT case_channels_archive_timestamp_check CHECK ((state = 'active' AND archived_at IS NULL) OR state = 'archived');

CREATE INDEX case_channels_case_state_idx ON case_channels(case_id, kind, state);

-- A composite key lets messages prove that their stored case id matches the
-- owning channel's case id.
ALTER TABLE case_channels
    ADD CONSTRAINT case_channels_id_case_id_unique UNIQUE (id, case_id);

CREATE OR REPLACE FUNCTION prevent_last_active_shared_channel()
RETURNS trigger AS $$
BEGIN
    IF OLD.kind = 'standard'
       AND OLD.state = 'active'
       AND NEW.state <> 'active'
       AND NOT EXISTS (
           SELECT 1 FROM case_channels other
           WHERE other.case_id = OLD.case_id
             AND other.kind = 'standard'
             AND other.state = 'active'
             AND other.id <> OLD.id
       ) THEN
        RAISE EXCEPTION 'cannot archive the last active shared channel';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER case_channels_keep_active_shared
BEFORE UPDATE OF state ON case_channels
FOR EACH ROW EXECUTE FUNCTION prevent_last_active_shared_channel();

CREATE OR REPLACE FUNCTION prevent_case_channel_delete()
RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'case message channels cannot be deleted';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER case_channels_prevent_delete
BEFORE DELETE ON case_channels
FOR EACH ROW EXECUTE FUNCTION prevent_case_channel_delete();

-- ---------------------------------------------------------------------------
-- Audience-aware audit metadata
-- ---------------------------------------------------------------------------
ALTER TABLE audit_log
    ADD COLUMN visibility TEXT NOT NULL DEFAULT 'shared';

ALTER TABLE audit_log
    ADD CONSTRAINT audit_log_visibility_check CHECK (visibility IN ('shared', 'volunteer_only'));

CREATE INDEX audit_log_entity_visibility_idx ON audit_log(entity_type, entity_id, visibility);

-- ---------------------------------------------------------------------------
-- Message/channel consistency and read evidence
-- ---------------------------------------------------------------------------
ALTER TABLE messages
    ADD CONSTRAINT messages_id_channel_case_unique UNIQUE (id, channel_id, case_id);

ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_channel_id_fkey;
ALTER TABLE messages
    ADD CONSTRAINT messages_channel_case_fkey
    FOREIGN KEY (channel_id, case_id)
    REFERENCES case_channels(id, case_id)
    ON DELETE RESTRICT;

CREATE TABLE message_read_receipts (
    message_id    TEXT NOT NULL,
    channel_id    TEXT NOT NULL,
    case_id       TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    first_read_at TIMESTAMPTZ,
    PRIMARY KEY (message_id, user_id),
    CONSTRAINT message_read_receipts_message_fkey
        FOREIGN KEY (message_id, channel_id, case_id)
        REFERENCES messages(id, channel_id, case_id)
        ON DELETE RESTRICT,
    CONSTRAINT message_read_receipts_channel_fkey
        FOREIGN KEY (channel_id, case_id)
        REFERENCES case_channels(id, case_id)
        ON DELETE RESTRICT
);

CREATE INDEX message_read_receipts_channel_user_idx ON message_read_receipts(channel_id, user_id);
CREATE INDEX message_read_receipts_message_idx ON message_read_receipts(message_id);
CREATE INDEX message_read_receipts_first_read_idx ON message_read_receipts(first_read_at);

-- Preserve existing unread facts without inventing read dates that were never
-- recorded. Existing rows remain unread/unknown until content is served later.
INSERT INTO message_read_receipts (message_id, channel_id, case_id, user_id, created_at, first_read_at)
SELECT n.message_id, n.channel_id, n.case_id, n.user_id, n.created_at, NULL
FROM channel_notifications n
ON CONFLICT DO NOTHING;

ALTER TABLE channel_notifications DROP CONSTRAINT IF EXISTS channel_notifications_channel_id_fkey;
ALTER TABLE channel_notifications DROP CONSTRAINT IF EXISTS channel_notifications_message_id_fkey;
ALTER TABLE channel_notifications
    ADD CONSTRAINT channel_notifications_channel_case_fkey
    FOREIGN KEY (channel_id, case_id)
    REFERENCES case_channels(id, case_id)
    ON DELETE CASCADE,
    ADD CONSTRAINT channel_notifications_message_channel_case_fkey
    FOREIGN KEY (message_id, channel_id, case_id)
    REFERENCES messages(id, channel_id, case_id)
    ON DELETE CASCADE;

-- ---------------------------------------------------------------------------
-- Message send throttle
-- ---------------------------------------------------------------------------
CREATE TABLE message_send_throttle (
    user_id      TEXT PRIMARY KEY,
    window_start TIMESTAMPTZ NOT NULL DEFAULT now(),
    send_count   INTEGER NOT NULL DEFAULT 0,
    last_sent_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
