-- Deactivating an account is a role, not a deletion. A `users` row cannot be
-- hard-deleted at all: `cases.owner_id` is ON DELETE RESTRICT and audit,
-- contact, and funding rows all point at it. So an account that should stop
-- being a login becomes `role = 'deactivated'`, following the same
-- archive-never-delete doctrine as case withdrawal (0029) and ADR-0002.
--
-- `users.role` has no CHECK constraint (0001), so the new slug needs no schema
-- change. The role-snapshot CHECKs in 0018 are deliberately untouched: a
-- deactivated account holds no session, so it can never author a note, and
-- existing snapshots keep the role their author held at the time.

-- What is true *because* an account holds the 'deactivated' role: who did it,
-- when, why, and the role to restore. This follows the rule 0015 set out --
-- `users` is the base record, and data that exists because of a role lives in a
-- table keyed by the user it extends -- rather than putting columns on every
-- user row for a state that applies to a handful of accounts.
--
-- One row per person, updated in place. A reactivated account keeps its row, so
-- the fact it was once deactivated survives, the same way `volunteers` keeps a
-- 'revoked' row instead of deleting the agreement.
--
-- Maintained by `users::set_deactivated`, which is the only writer, alongside
-- the invariant 0015 already maintains in `users::set_role_in`:
--   users.role = 'deactivated'  <=>  a row here with reactivated_at IS NULL
CREATE TABLE account_deactivations (
    user_id             TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    -- The role held before deactivation, so reactivation is lossless. The part
    -- `cases.status_before_withdrawal` plays for a withdrawn case.
    previous_role       TEXT        NOT NULL,
    -- Optional context an admin typed, shown to whoever decides whether to
    -- restore. Never required, so cleaning up an obvious duplicate needs no essay.
    reason              TEXT        NOT NULL DEFAULT '',
    -- Who decided, and when. `deactivated_by_name` is denormalized for display
    -- so a later change to the actor's own account does not erase the record --
    -- the `volunteers.decided_by_name` pattern.
    deactivated_by      TEXT        REFERENCES users(id) ON DELETE SET NULL,
    deactivated_by_name TEXT        NOT NULL DEFAULT '',
    deactivated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Set when the account is restored; NULL means still deactivated.
    reactivated_by_name TEXT        NOT NULL DEFAULT '',
    reactivated_at      TIMESTAMPTZ,
    -- A stored deactivation can always be explained and always be restored.
    CONSTRAINT account_deactivations_complete CHECK (
        previous_role <> '' AND deactivated_by_name <> ''
    )
);

-- The hot read is the admin's "Deactivated accounts" section, which wants the
-- currently-deactivated rows newest first.
CREATE INDEX account_deactivations_active_idx
    ON account_deactivations (deactivated_at DESC) WHERE reactivated_at IS NULL;
