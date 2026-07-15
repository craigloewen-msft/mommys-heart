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
