-- Brute-force / abuse throttling for auth endpoints.
--
-- The email OTP second factor already stops account takeover, but the password
-- check (`login`) and the password-reset request endpoint were previously
-- unthrottled: an attacker could hammer `login` to confirm a valid password via
-- the differing response, or spam `request_password_reset` to email-bomb a
-- victim. This table records consecutive failed/abusive attempts and
-- temporarily locks the identifier after too many, auto-expiring so a
-- legitimate user is only briefly delayed.
--
-- Counters are namespaced by `scope` (e.g. 'login', 'password_reset') so
-- unrelated flows keep separate budgets and cannot lock one another out. The
-- identifier is the normalized (lowercased) email; rows are created lazily on
-- the first failure and deleted on a successful sign-in.
CREATE TABLE auth_throttle (
    scope          TEXT NOT NULL,
    identifier     TEXT NOT NULL,
    fail_count     INTEGER NOT NULL DEFAULT 0,
    last_failed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until   TIMESTAMPTZ,
    PRIMARY KEY (scope, identifier)
);
