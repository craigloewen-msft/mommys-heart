-- Consolidated initial schema for the Mommy's Heart CRM domain.
--
-- This single migration is the clean starting point for the new database; it
-- folds together everything the previous incremental migrations built up:
-- core CRM tables, file-backed evidence, the audit log with its machine-readable
-- retention timestamp, per-user settings, email-OTP MFA / device trust / password
-- reset, and auth throttling.
--
-- Enums (AccountRole, CaseStatus, CaseCapability) are stored as their text
-- `slug()` values so the Rust `from_slug()` helpers round-trip cleanly. IDs are
-- text (e.g. "u-admin", "c-1001") to match the existing domain model.

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    first_name    TEXT NOT NULL DEFAULT '',
    last_name     TEXT NOT NULL DEFAULT '',
    email         TEXT NOT NULL UNIQUE,
    phone         TEXT NOT NULL DEFAULT '',
    home_address  TEXT NOT NULL DEFAULT '',
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL
);

CREATE TABLE grants (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE cases (
    id       TEXT PRIMARY KEY,
    name     TEXT NOT NULL,
    status   TEXT NOT NULL,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX cases_owner_id_idx ON cases(owner_id);

-- Ordered key/value properties on a case.
CREATE TABLE case_properties (
    case_id TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    ord     INTEGER NOT NULL,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (case_id, ord)
);

CREATE TABLE case_notes (
    id         TEXT PRIMARY KEY,
    case_id    TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    author     TEXT NOT NULL,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    -- Monotonic sequence to preserve insertion order (timestamps are display strings).
    seq        BIGSERIAL
);

CREATE INDEX case_notes_case_id_idx ON case_notes(case_id);

-- File-backed evidence: the DB keeps metadata + a pointer to the blob in Azure
-- Blob Storage, never the bytes. Metadata-only rows are valid (empty `blob_path`).
--
--   original_filename  the user-supplied file name (display + download name)
--   content_type       the validated MIME type (e.g. application/pdf)
--   size_bytes         file size, for display and quota/limit checks
--   sha256             hex SHA-256 of the bytes, for integrity + future dedup
--   blob_path          blob name within the evidence container (empty = no file)
--   status             lifecycle: 'stored' (default), plus forward-compatible
--                      'pending' / 'quarantined' / 'infected' hooks so malware
--                      scanning (e.g. Microsoft Defender for Storage) can be
--                      layered on later without another migration.
CREATE TABLE evidence (
    id                TEXT PRIMARY KEY,
    case_id           TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    uploaded_by       TEXT NOT NULL,
    uploaded_at       TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    original_filename TEXT   NOT NULL DEFAULT '',
    content_type      TEXT   NOT NULL DEFAULT '',
    size_bytes        BIGINT NOT NULL DEFAULT 0,
    sha256            TEXT   NOT NULL DEFAULT '',
    blob_path         TEXT   NOT NULL DEFAULT '',
    status            TEXT   NOT NULL DEFAULT 'stored',
    seq               BIGSERIAL
);

CREATE INDEX evidence_case_id_idx ON evidence(case_id);

CREATE TABLE messages (
    id        TEXT PRIMARY KEY,
    case_id   TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL,
    author    TEXT NOT NULL,
    body      TEXT NOT NULL,
    sent_at   TEXT NOT NULL,
    seq       BIGSERIAL
);

CREATE INDEX messages_case_id_idx ON messages(case_id);

-- Per-user, per-case capability grants (many-to-many, one row per capability).
CREATE TABLE case_assignments (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    case_id    TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    capability TEXT NOT NULL,
    PRIMARY KEY (user_id, case_id, capability)
);

CREATE INDEX case_assignments_case_id_idx ON case_assignments(case_id);

-- Unified append-only audit log for both users and cases. `at` is a
-- human-readable display string (`YYYY-MM-DD HH:MM`, local time); `created_at`
-- is the machine-readable, timezone-aware source of truth used for retention.
CREATE TABLE audit_log (
    id          TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,          -- 'user' | 'case'
    entity_id   TEXT NOT NULL,
    actor       TEXT NOT NULL,
    field       TEXT NOT NULL,
    old_value   TEXT NOT NULL DEFAULT '',
    new_value   TEXT NOT NULL DEFAULT '',
    at          TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Monotonic sequence so "newest first" ordering is stable.
    seq         BIGSERIAL
);

CREATE INDEX audit_log_entity_idx ON audit_log(entity_type, entity_id);
-- Keep the retention DELETE cheap.
CREATE INDEX audit_log_created_at_idx ON audit_log(created_at);

-- Opaque session tokens (only the SHA-256 hash is stored).
CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_user_id_idx ON sessions(user_id);

-- Shared monotonic id source for newly created records (replaces the in-memory
-- `seq` counter that produced ids like "c-5001", "n-5002", "cl-5003", ...).
CREATE SEQUENCE app_id_seq START 5001;

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

-- Email-OTP multi-factor authentication, "remember this device" trust, and
-- self-service password reset.
--
-- Like the `sessions` table, every user-facing token here is opaque: only its
-- SHA-256 hash is stored, so a database leak cannot be replayed. Codes and
-- tokens are single-user-scoped and short-lived; expired rows are pruned
-- opportunistically by the application.

-- A pending second-factor challenge created after a correct password, before a
-- session is minted. `challenge_hash` is the hash of the opaque token handed to
-- the browser in the short-lived `mfa` cookie; `code_hash` is the hash of the
-- 6-digit code emailed to the user. `attempts` caps guessing.
CREATE TABLE mfa_challenges (
    challenge_hash TEXT PRIMARY KEY,
    user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash      TEXT NOT NULL,
    attempts       INTEGER NOT NULL DEFAULT 0,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at     TIMESTAMPTZ NOT NULL
);

CREATE INDEX mfa_challenges_user_id_idx ON mfa_challenges(user_id);

-- A browser the user chose to trust ("remember this device"): while a live row
-- exists for the presented cookie *and* it belongs to the authenticating user,
-- the OTP step is skipped. Scoped per user so a stolen token cannot unlock a
-- different account.
CREATE TABLE trusted_devices (
    token_hash TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX trusted_devices_user_id_idx ON trusted_devices(user_id);

-- A single-use password-reset token delivered as a link. `used` flips true once
-- consumed so a link cannot be replayed even before it expires.
CREATE TABLE password_reset_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    used       BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX password_reset_tokens_user_id_idx ON password_reset_tokens(user_id);

-- Brute-force / abuse throttling for auth endpoints. Counters are namespaced by
-- `scope` (e.g. 'login', 'password_reset') so unrelated flows keep separate
-- budgets and cannot lock one another out. The identifier is the normalized
-- (lowercased) email; rows are created lazily on the first failure and deleted
-- on a successful sign-in.
CREATE TABLE auth_throttle (
    scope          TEXT NOT NULL,
    identifier     TEXT NOT NULL,
    fail_count     INTEGER NOT NULL DEFAULT 0,
    last_failed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until   TIMESTAMPTZ,
    PRIMARY KEY (scope, identifier)
);
