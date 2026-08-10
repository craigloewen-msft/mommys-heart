-- The volunteer's own details, collected with the agreement they sign: skills,
-- date of birth, contact, emergency contact, and an optional SSN.
--
-- These live on `volunteers` rather than in a table of their own because there
-- is exactly one row per person already and this information is part of the
-- same accepted application: the agreement and what was submitted with it are
-- one record, and a decision or a re-acceptance updates it in place.
ALTER TABLE volunteers
    ADD COLUMN skills_focus           TEXT NOT NULL DEFAULT '',
    -- Nullable, unlike the text columns: there is no sensible "empty date", and
    -- NULL says "predates this form" exactly as '' does for the others.
    ADD COLUMN date_of_birth          DATE,
    -- Digits only. Never selected by the ordinary read path: `db::volunteers`
    -- omits this column from SELECT_COLUMNS and exposes only `ssn <> ''` as a
    -- boolean, so the number cannot reach a browser by accident. The single
    -- raw read is `db::volunteers::ssn`, called by one site-admin-only server
    -- function that audits every disclosure.
    ADD COLUMN ssn                    TEXT NOT NULL DEFAULT '',
    -- The phone number as given on the accepted application. Deliberately a
    -- copy of `users.phone` rather than a join: the account's number may change
    -- later, and this records what was submitted at the time of acceptance.
    ADD COLUMN phone                  TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_first_name   TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_last_name    TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_relationship TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_phone        TEXT NOT NULL DEFAULT '';
