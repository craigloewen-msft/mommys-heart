-- Durable record of outbound email delivery failures.
--
-- Every real send goes through `server::email::send_email`, called from the two
-- higher-level flows (`server::notifications` for case activity and
-- `server::email::auth_notifications` for MFA / verification / password-reset
-- mail). When a send ultimately fails, the calling flow appends a row here so
-- the failure is visible in the admin dashboard rather than only in the server
-- logs. Mirrors `audit_log`: append-only, newest-first via a monotonic `seq`,
-- with a machine-readable `created_at` driving retention.

CREATE TABLE email_failures (
    id          TEXT PRIMARY KEY,
    recipient   TEXT NOT NULL,          -- the address we tried to email
    subject     TEXT NOT NULL DEFAULT '',
    context     TEXT NOT NULL DEFAULT '',   -- e.g. 'Case notification' | 'Authentication email'
    error       TEXT NOT NULL,          -- the final error surfaced by send_email
    at          TEXT NOT NULL,          -- display timestamp (YYYY-MM-DD HH:MM, local)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Monotonic sequence so "newest first" ordering is stable.
    seq         BIGSERIAL
);

-- Keep the retention DELETE cheap.
CREATE INDEX email_failures_created_at_idx ON email_failures(created_at);
