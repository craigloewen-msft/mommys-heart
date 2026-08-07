-- Fold the review decision into `cases.status`, so a case has exactly one state.
--
-- The decision and the lifecycle are not two independent things. A case that has
-- not been accepted is not "Open", and a declined case is not "Open" either --
-- of the nine combinations a separate `review_state` column would allow, only
-- three mean anything, and "Open + Declined" is representable nonsense the UI
-- would eventually show someone.
--
-- (This is *not* the `visibility` / `section` split from 0004. Those are
-- genuinely orthogonal: every combination of who-may-see-it and how-it-is-
-- grouped is meaningful. Here the cross product is mostly meaningless, which is
-- the signature of one dimension modelled as two.)
--
-- So the lifecycle grows two states at its ends:
--
--   pending_review --accept--> open <-> monitor --> closed
--         |
--         +--------decline--> declined   (terminal, carries a reason)
--
-- Who may make a transition still differs -- only an operations/site admin may
-- accept or decline, while staff move a case between open/monitor/closed -- but
-- that is a rule about transitions, enforced in the server functions, not a
-- reason for a second column.
--
-- Existing rows keep their current status untouched: every case already in the
-- system predates review and is live work. Defaulting them into 'pending_review'
-- would drop live cases into limbo and tell their clients that work they can see
-- happening had never been approved.
ALTER TABLE cases
    -- Why a case was declined. Shown to the client verbatim, so a decline is
    -- never an unexplained dead end. Required (non-empty) on decline, enforced
    -- in the server function rather than here so the message can be a sentence
    -- rather than a constraint violation. Empty for every other state.
    ADD COLUMN review_reason TEXT NOT NULL DEFAULT '',
    -- Who decided, and when. Display strings, matching the `now_stamp()` format
    -- the rest of the schema stores ('YYYY-MM-DD HH:MM'). Empty means no
    -- decision has been recorded -- true for every pre-existing row.
    ADD COLUMN reviewed_by   TEXT NOT NULL DEFAULT '',
    ADD COLUMN reviewed_at   TEXT NOT NULL DEFAULT '';

-- The hot read is the admin Case Requests tab and its count badge, both of which
-- ask for exactly one status.
CREATE INDEX cases_status_idx ON cases(status);
