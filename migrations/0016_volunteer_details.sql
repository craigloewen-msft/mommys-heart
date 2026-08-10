-- The volunteer's own details, collected with the agreement they accept:
-- skills, date of birth, contact, emergency contact, and an optional SSN.
-- These live on `volunteers` because there is one row per person already and
-- this is part of the same accepted application.
ALTER TABLE volunteers
    ADD COLUMN skills_focus           TEXT NOT NULL DEFAULT '',
    -- Nullable, unlike the text columns: there is no sensible "empty date".
    ADD COLUMN date_of_birth          DATE,
    -- Digits only, and never selected by the ordinary read path: `db::volunteers`
    -- omits it from SELECT_COLUMNS and exposes only `ssn <> ''`. The one raw
    -- read is `db::volunteers::reveal_ssn`, which audits every disclosure.
    ADD COLUMN ssn                    TEXT NOT NULL DEFAULT '',
    -- The phone as given on the accepted application; `users.phone` may change
    -- later, so this records what was submitted at the time.
    ADD COLUMN phone                  TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_first_name   TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_last_name    TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_relationship TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_phone        TEXT NOT NULL DEFAULT '';
