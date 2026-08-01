-- Case properties and case files gain two columns, so a case can carry both the
-- client-facing record and the team's own working record (the intake / outtake
-- paperwork) side by side.
--
--   visibility — WHO may see this row.
--   section    — how the row is grouped for display.
--
-- They are separate on purpose. Grouping a row under "Intake" says nothing
-- about who may read it, and hiding a row from the client says nothing about
-- where it appears. Fusing the two into one "internal" flag is what makes such
-- a column impossible to reason about later.

-- ---------------------------------------------------------------------------
-- visibility
--
-- Reuses the vocabulary the case chat already established in
-- `case_channels.kind` (see 0003), so the app has ONE word for "staff only"
-- rather than a new synonym per feature:
--
--   'shared'         everyone who can view the case, clients included.
--   'volunteer_only' volunteers and admins working the case; a client account
--                    must never see these rows.
--
-- Both tables use the same two values on purpose: one question, one vocabulary.
--
-- This is an account-role rule, exactly like the volunteer-only chat channel:
-- it is orthogonal to the per-case capabilities in `case_assignments`, which
-- decide whether you may touch a case's properties or files at all. Visibility
-- then decides which of them you see. No new capability is introduced.
--
-- Defaulting to 'shared' leaves every pre-existing row exactly as visible as it
-- was before this migration.
-- ---------------------------------------------------------------------------
ALTER TABLE case_properties
    ADD COLUMN visibility TEXT NOT NULL DEFAULT 'shared';

ALTER TABLE evidence
    ADD COLUMN visibility TEXT NOT NULL DEFAULT 'shared';

-- `ord` becomes a per-visibility display order, so each list keeps its own
-- 0..n sequence and rewriting one list never renumbers the other.
ALTER TABLE case_properties DROP CONSTRAINT case_properties_pkey;
ALTER TABLE case_properties ADD PRIMARY KEY (case_id, visibility, ord);

-- The hot read is "the rows on this case that this viewer is allowed to see".
CREATE INDEX evidence_case_visibility_idx ON evidence(case_id, visibility);

-- ---------------------------------------------------------------------------
-- section
--
-- A free-text grouping label used purely for display: 'Intake', 'Outtake', or
-- anything a future workflow needs. Empty means ungrouped, which is what every
-- existing row is.
--
-- This is the whole reason intake/outtake needs no schema of its own. A case is
-- created with a set of named, empty rows — an `evidence` row with an empty
-- `blob_path` and `status = 'awaiting'` is a file nobody has provided yet, and
-- a `case_properties` row with an empty `value` is a property nobody has filled
-- in yet. Prepopulating a case is therefore an ordinary INSERT of ordinary
-- rows, and the list of what to insert lives in application code where it can
-- be reviewed and changed by deploy.
--
-- Nothing here names 'Intake' or 'Outtake': they are values, not structure.
-- Renaming, reordering, adding a section, or changing a prepopulated row's
-- audience is a code change, never a migration.
-- ---------------------------------------------------------------------------
ALTER TABLE case_properties
    ADD COLUMN section TEXT NOT NULL DEFAULT '';

ALTER TABLE evidence
    ADD COLUMN section TEXT NOT NULL DEFAULT '';
