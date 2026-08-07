-- Case files become a folder tree.
--
-- Until now a file carried a free-text `section` label and a `visibility`, and
-- the case view grouped by the two. That is a flat list wearing a folder
-- costume: you cannot nest it, you cannot move a file without retyping a label,
-- and the blob container stays a pile of opaque ids.
--
-- `case_folders` makes the grouping a real thing rows point at.
--
--   * Every case gets exactly two top-level folders, one per audience ("Volunteer
--     only" and "Shared with client"). They are created with the case and are
--     the only place `visibility` is decided.
--   * Anything below a root inherits that root's audience, so a file can never
--     end up in a folder whose audience disagrees with its own — moving a file
--     into a folder is what changes who can see it.
--   * `evidence.visibility` stays, denormalized from the folder, so every read
--     path that filters by audience keeps working without a join. The
--     application only ever writes it from the owning folder.
CREATE TABLE case_folders (
    id         TEXT PRIMARY KEY,
    case_id    TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    -- NULL marks one of the two standing top-level folders.
    parent_id  TEXT REFERENCES case_folders(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    visibility TEXT NOT NULL,
    -- Monotonic sequence so folders list in creation order.
    seq        BIGSERIAL
);

CREATE INDEX case_folders_case_id_idx ON case_folders(case_id);

-- One folder per name inside a parent, case-insensitively. COALESCE gives the
-- two roots — whose parent is NULL — the same guarantee.
CREATE UNIQUE INDEX case_folders_name_idx
    ON case_folders(case_id, COALESCE(parent_id, ''), lower(name));

ALTER TABLE evidence
    ADD COLUMN folder_id TEXT REFERENCES case_folders(id) ON DELETE CASCADE;

-- ---------------------------------------------------------------------------
-- Backfill: rebuild the existing grouping as folders.
--
-- Ids come from the same `app_id_seq` the application uses, so migrated rows
-- are indistinguishable from ones created at runtime.
-- ---------------------------------------------------------------------------

-- The two standing roots, for every case that already exists.
INSERT INTO case_folders (id, case_id, parent_id, name, visibility)
SELECT 'f-' || nextval('app_id_seq'), c.id, NULL, 'Volunteer only', 'volunteer_only'
FROM cases c;

INSERT INTO case_folders (id, case_id, parent_id, name, visibility)
SELECT 'f-' || nextval('app_id_seq'), c.id, NULL, 'Shared with client', 'shared'
FROM cases c;

-- Each distinct non-empty section becomes a sub-folder of the root for its
-- audience; files with no section stay at the top of that root.
INSERT INTO case_folders (id, case_id, parent_id, name, visibility)
SELECT 'f-' || nextval('app_id_seq'), s.case_id, root.id, s.section, s.visibility
FROM (SELECT DISTINCT case_id, visibility, section FROM evidence WHERE section <> '') s
JOIN case_folders root
  ON root.case_id = s.case_id AND root.parent_id IS NULL AND root.visibility = s.visibility;

UPDATE evidence e
SET folder_id = f.id
FROM case_folders f
WHERE f.case_id = e.case_id
  AND f.visibility = e.visibility
  AND CASE
        WHEN e.section = '' THEN f.parent_id IS NULL
        ELSE f.parent_id IS NOT NULL AND f.name = e.section
      END;

-- Any row whose visibility was never one of the two known audiences would have
-- been left behind; there is nowhere sensible to put it, so it lands in the
-- volunteer-only root where only the team can see it.
UPDATE evidence e
SET folder_id = f.id, visibility = 'volunteer_only'
FROM case_folders f
WHERE e.folder_id IS NULL
  AND f.case_id = e.case_id
  AND f.parent_id IS NULL
  AND f.visibility = 'volunteer_only';

ALTER TABLE evidence ALTER COLUMN folder_id SET NOT NULL;

CREATE INDEX evidence_folder_id_idx ON evidence(folder_id);

-- The folder replaces it: grouping is now a row you point at, not a label you
-- retype. `blob_path` is untouched by any of this — a blob is named after the
-- evidence id, so where a file sits in the tree never moves its bytes.
ALTER TABLE evidence DROP COLUMN section;