-- `users` becomes a base record and the two account kinds that carry their own
-- data get a table keyed by the user they extend. `users.role` stays the source
-- of truth for access; these tables hold what is true *because* of that role.

-- A volunteer's agreement and their application to become one: one row per
-- person, so a decision updates it in place. Enforced in `users::set_role_in`:
--   users.role = 'volunteer'  <=>  a row here with status = 'approved'
-- 'revoked' records someone who lost the role without deleting their agreement.
CREATE TABLE volunteers (
    user_id           TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    status            TEXT        NOT NULL CHECK (status IN ('pending', 'approved', 'denied', 'revoked')),
    -- Which wording they accepted, stored verbatim so revising the agreement
    -- never rewrites what a past volunteer agreed to. Empty for the rows
    -- backfilled below, whose volunteers predate the agreement entirely.
    agreement_version TEXT        NOT NULL,
    agreed_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Who decided, and when. `decided_by_name` is denormalized for display so a
    -- deleted admin account does not erase the record of who approved someone.
    decided_by        TEXT        REFERENCES users(id) ON DELETE SET NULL,
    decided_by_name   TEXT        NOT NULL DEFAULT '',
    decision_note     TEXT        NOT NULL DEFAULT '',
    decided_at        TIMESTAMPTZ
);

-- The hot read is the admin's pending-application queue and its count badge.
CREATE INDEX volunteers_pending_idx ON volunteers (agreed_at) WHERE status = 'pending';

-- The client side of the same split, deliberately thin for now.
--   users.role = 'client'  =>  a row here
-- One-way: this table has no status, so a retained row is history, not a clash.
CREATE TABLE clients (
    user_id    TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One-time backfill so users that predate these tables get their subtype row;
-- new rows come from `seed.rs` and `users::set_role_in`. Empty
-- `agreement_version` records a volunteer who never saw the agreement.
INSERT INTO volunteers (user_id, status, agreement_version, decided_by_name)
SELECT id, 'approved', '', 'Migration'
FROM users
WHERE role = 'volunteer'
ON CONFLICT (user_id) DO NOTHING;

INSERT INTO clients (user_id)
SELECT id FROM users WHERE role = 'client'
ON CONFLICT (user_id) DO NOTHING;
