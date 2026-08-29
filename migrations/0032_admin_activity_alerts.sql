-- Admin activity alerts: a digest of what the audit log already records.
--
-- The per-user opt-in joins the other notification categories in `user_settings`
-- (see `server_fns::settings::NotificationKind`), defaulting to true like every
-- category before it.
ALTER TABLE user_settings
    ADD COLUMN notification_admin_activity BOOLEAN NOT NULL DEFAULT true;

-- The activity feed is a *view* over `audit_log` (plus the restricted
-- `case_note_audit_log`), not a second copy of those facts. All this table holds
-- is how far the digest has got: one row, naming the last `seq` mailed from each
-- source. Reading the feed never writes, and the digest never writes to an
-- append-only audit table.
CREATE TABLE admin_activity_digest_state (
    id                 BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    last_audit_seq     BIGINT NOT NULL DEFAULT 0,
    last_note_seq      BIGINT NOT NULL DEFAULT 0,
    last_sent_at       TIMESTAMPTZ
);

-- Start the watermark at the current end of each log, so the first digest after
-- deployment reports genuinely new activity rather than replaying all history.
INSERT INTO admin_activity_digest_state (id, last_audit_seq, last_note_seq)
VALUES (
    true,
    COALESCE((SELECT max(seq) FROM audit_log), 0),
    COALESCE((SELECT max(seq) FROM case_note_audit_log), 0)
);

-- The feed and digest both scan recent rows across every entity, which neither
-- log was indexed for: their indexes lead with entity/case id.
CREATE INDEX audit_log_seq_idx ON audit_log(seq DESC);
CREATE INDEX case_note_audit_seq_idx ON case_note_audit_log(seq DESC);
