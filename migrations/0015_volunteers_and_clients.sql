-- `users` becomes a base record and the two account kinds that carry their own
-- data get a table of their own, keyed by the user they extend. Role still lives
-- on `users` -- these tables hold what is true *because* someone is a volunteer
-- or a client, not the fact itself.

-- A volunteer's agreement and their application to become one. There is exactly
-- one row per person: the row IS the application, so a decision updates it in
-- place rather than filing a second record. A denied or revoked applicant may
-- accept the agreement again, which overwrites the row back to 'pending'.
--
-- The invariant this table exists to carry, enforced in `users::apply_role_in`:
--   users.role = 'volunteer'  <=>  a row here with status = 'approved'
-- 'revoked' is how someone who was approved and later lost the role is recorded
-- without deleting the agreement they genuinely accepted.
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

-- The client side of the same split. Deliberately thin: it exists so client-only
-- data has a home and the structure is symmetric with `volunteers`.
--
--   users.role = 'client'  =>  a row here
-- One-way on purpose: this table carries no status, so a row retained for
-- someone who has since become a volunteer is history, not a contradiction.
CREATE TABLE clients (
    user_id    TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Repair any database that already has users: give every existing volunteer and
-- client their subtype row. New databases are seeded consistent instead (see
-- `seed.rs`), so this is a backfill for existing data, not the only mechanism.
-- Existing volunteers are approved with an empty `agreement_version`: they are
-- volunteers, but they never saw the agreement, which is what empty records.
INSERT INTO volunteers (user_id, status, agreement_version, decided_by_name)
SELECT id, 'approved', '', 'Migration'
FROM users
WHERE role = 'volunteer'
ON CONFLICT (user_id) DO NOTHING;

INSERT INTO clients (user_id)
SELECT id FROM users WHERE role = 'client'
ON CONFLICT (user_id) DO NOTHING;
