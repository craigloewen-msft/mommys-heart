-- Add a real timestamp to the audit log so retention (pruning old entries) can
-- be computed reliably. The existing `at` column is a human-readable display
-- string (`YYYY-MM-DD HH:MM`, local time) and is kept for display; `created_at`
-- is the machine-readable, timezone-aware source of truth used for retention.
--
-- Existing rows backfill to `now()` via the column default, so nothing is pruned
-- until it is genuinely older than the retention window.

ALTER TABLE audit_log
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Keep the retention DELETE cheap.
CREATE INDEX audit_log_created_at_idx ON audit_log(created_at);
