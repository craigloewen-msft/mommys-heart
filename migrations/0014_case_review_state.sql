-- Whether the organization has *decided to take* a case, kept deliberately
-- separate from `cases.status`.
--
-- These answer two different questions, and fusing them is exactly the mistake
-- migration 0004 avoided when it split `visibility` from `section`:
--
--   status        where a case is in its life: Open / Monitor / Closed. Staff
--                 own it, and it says nothing about whether we accepted it.
--   review_state  has an operations/site admin accepted this case? Admin-only,
--                 and the only trustworthy signal of an organizational decision.
--
-- `status` cannot serve as the decision signal: it is guarded by the per-case
-- `edit_case` capability, and the public signup flow (0010) grants the client
-- owner the full capability set -- so a client can change their own case's
-- status. A decision the subject of the case can overwrite is not a decision.
--
-- A third question -- "is anyone actually working this case?" -- deliberately
-- gets NO column here. It is derived at read time from `review_state` plus the
-- existing `case_assignments` rows, so assigning a volunteer updates every
-- surface at once and the two can never drift out of sync.
--
-- Values are text slugs matching the Rust `from_slug()` round-trip convention
-- used by every other enum in this schema: 'pending_review', 'accepted',
-- 'declined'.
--
-- Existing rows default to 'accepted': every case already in the system predates
-- review and is being worked on today. Defaulting them to 'pending_review' would
-- retroactively drop live cases into limbo and tell their clients that work they
-- can see happening has not been approved.
ALTER TABLE cases
    ADD COLUMN review_state  TEXT NOT NULL DEFAULT 'accepted',
    -- Why a case was declined. Shown to the client verbatim, so a decline is
    -- never an unexplained dead end. Required (non-empty) on decline, enforced
    -- in the server function rather than here so the message can be a sentence
    -- rather than a constraint violation.
    ADD COLUMN review_reason TEXT NOT NULL DEFAULT '',
    -- Who decided, and when. Display strings, matching the `now_stamp()` format
    -- the rest of the schema stores ('YYYY-MM-DD HH:MM'). Empty means no
    -- decision has been recorded -- true for every pre-existing row.
    ADD COLUMN reviewed_by   TEXT NOT NULL DEFAULT '',
    ADD COLUMN reviewed_at   TEXT NOT NULL DEFAULT '';

-- The hot read is the admin Cases tab filtering to what still needs a decision,
-- plus the pending count badge.
CREATE INDEX cases_review_state_idx ON cases(review_state);
