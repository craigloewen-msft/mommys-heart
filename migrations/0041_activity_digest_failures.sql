-- Email delivery failures in the admin activity digest.
--
-- The digest reports failures from `email_failures` alongside audit activity, so
-- it needs a third watermark: how far it has mailed through that log. Same shape
-- as the other two in this one-row table (see 0032).
ALTER TABLE admin_activity_digest_state
    ADD COLUMN last_failure_seq BIGINT NOT NULL DEFAULT 0;

-- Start at the current end of the log, so the first digest after deployment
-- reports genuinely new failures rather than replaying all history.
UPDATE admin_activity_digest_state
SET last_failure_seq = COALESCE((SELECT max(seq) FROM email_failures), 0);
