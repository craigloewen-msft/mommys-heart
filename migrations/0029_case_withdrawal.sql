-- Let a case owner withdraw a case, without deleting anything.
--
-- A case row cannot be deleted: it cascades to `case_channels` and `case_notes`,
-- both of which refuse `DELETE` outright (0017, 0018). Withdrawal is therefore a
-- status, following the archive-never-delete doctrine in ADR-0002.
ALTER TABLE cases
    -- Who withdrew it, and when. Display strings matching the `now_stamp()`
    -- format the rest of the schema stores ('YYYY-MM-DD HH:MM'). Empty means
    -- never withdrawn -- true for every pre-existing row.
    ADD COLUMN withdrawn_by             TEXT NOT NULL DEFAULT '',
    ADD COLUMN withdrawn_at             TEXT NOT NULL DEFAULT '',
    -- Optional context the owner typed, shown to the admin deciding whether to
    -- restore. Never required, so a mistake needs no explanation.
    ADD COLUMN withdrawal_reason        TEXT NOT NULL DEFAULT '',
    -- The status held before withdrawal, so an admin restore is lossless and
    -- returns the case to exactly where it was.
    ADD COLUMN status_before_withdrawal TEXT NOT NULL DEFAULT '';

-- A withdrawn case always knows when it happened and what to restore it to, so
-- the state can never become unexplainable or unrecoverable.
ALTER TABLE cases
    ADD CONSTRAINT cases_withdrawal_complete CHECK (
        status <> 'withdrawn'
        OR (withdrawn_at <> '' AND status_before_withdrawal <> '')
    );
