-- Fold the review decision into `cases.status`, so a case has exactly one state.
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
