-- Initial schema for the Mommy's Heart CRM domain.
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

CREATE TABLE evidence (
    id          TEXT PRIMARY KEY,
    case_id     TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    uploaded_by TEXT NOT NULL,
    uploaded_at TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    seq         BIGSERIAL
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

-- Unified append-only audit log for both users and cases.
CREATE TABLE audit_log (
    id          TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,          -- 'user' | 'case'
    entity_id   TEXT NOT NULL,
    actor       TEXT NOT NULL,
    field       TEXT NOT NULL,
    old_value   TEXT NOT NULL DEFAULT '',
    new_value   TEXT NOT NULL DEFAULT '',
    at          TEXT NOT NULL,
    -- Monotonic sequence so "newest first" ordering is stable.
    seq         BIGSERIAL
);

CREATE INDEX audit_log_entity_idx ON audit_log(entity_type, entity_id);

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
