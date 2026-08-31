-- Case documents move to a SharePoint document library.
--
-- The evidence feature stored file *bytes* in an Azure Blob container and
-- mirrored the folder tree in `case_folders`, with one `evidence` row per file.
-- It has been switched off since migration-era task 00030. Its replacement puts
-- the files in a SharePoint document library and keeps only two things here:
--
--   * where a case's folder is (`cases.drive_item_id`), so the app can find it
--     again without searching the library by name; and
--   * which sharing invitations we issued (`case_document_permissions`), so a
--     revoke can delete the exact permission it granted.
--
-- Everything else -- the subfolders, the files, their sizes and timestamps --
-- is read live from the library. There is deliberately no mirror table: two
-- copies of a file listing drift apart, and the one in SharePoint is the one
-- people actually edit.

ALTER TABLE cases
    -- Graph's id for the case's folder. Empty until provisioning succeeds,
    -- which is what lets a case be created while the library is unreachable.
    ADD COLUMN drive_item_id TEXT NOT NULL DEFAULT '',
    -- The browser link to that folder, shown as "Open in SharePoint".
    ADD COLUMN documents_web_url TEXT NOT NULL DEFAULT '';

-- One row per sharing invitation the app has issued, so granting and revoking
-- are exact rather than best-guess.
--
-- Invitations are issued on a case's *top-level* folders, never on the case
-- root: the top-level folders are what carry an audience, so inviting a client
-- to the case root would hand them the volunteer-only paperwork too.
-- `folder_name` is therefore part of the key.
CREATE TABLE case_document_permissions (
    case_id       TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The top-level folder the invitation was issued on, e.g. 'Intake'.
    folder_name   TEXT NOT NULL,
    -- Graph's permission id, needed to revoke exactly what we granted.
    permission_id TEXT NOT NULL,
    -- 'read' or 'write', mirroring whether the user may upload.
    role          TEXT NOT NULL,
    -- The address invited, kept so a revoke still works after a user changes
    -- their email (the old grant is against the old address).
    email         TEXT NOT NULL,
    granted_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (case_id, user_id, folder_name)
);

CREATE INDEX case_document_permissions_user_idx
    ON case_document_permissions(user_id);

-- The blob-backed implementation, dropped rather than left to rot. The library
-- is the source of truth for case files now, so these tables cannot be read
-- back into anything and keeping them would only invite a second write path.
--
-- `case_folders` goes with them: a case's standing folders are declared in code
-- (`helpers::new_case_folders::NEW_CASE_FOLDERS`) and created in the library,
-- so a table repeating them is one more thing to keep in step for no gain.
DROP TABLE IF EXISTS evidence_scan_log;
DROP TABLE IF EXISTS evidence;
DROP TABLE IF EXISTS case_folders;
