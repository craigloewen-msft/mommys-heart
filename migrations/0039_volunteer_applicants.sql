-- A volunteer application filed by somebody who has no account yet.
--
-- `volunteers` is keyed by `users(id)`, so it cannot hold an applicant: the
-- public volunteer signup has to stage the whole signed agreement somewhere
-- before any account exists, exactly as `pending_registrations` stages a client
-- signup. A `users` row is created only when the approved applicant follows
-- their setup link and chooses a password, at which point the details below are
-- copied into `volunteers` and this row is marked 'completed'.
CREATE TABLE volunteer_applicants (
    id                TEXT PRIMARY KEY,
    first_name        TEXT NOT NULL,
    last_name         TEXT NOT NULL,
    -- The address they applied with. Every notification goes here, including the
    -- setup link, even when an admin assigns a different sign-in address.
    email             TEXT NOT NULL,

    -- The agreement they accepted, stored verbatim like `volunteers` does.
    agreement_version TEXT        NOT NULL,
    agreed_at         TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- The details submitted with it, mirroring the `volunteers` columns so the
    -- completion step is a straight copy.
    skills_focus           TEXT NOT NULL DEFAULT '',
    volunteer_role         TEXT NOT NULL DEFAULT '',
    date_of_birth          DATE,
    -- Digits only, and never selected by the ordinary read path, exactly as on
    -- `volunteers`: it is moved across at completion and cleared here.
    ssn                    TEXT NOT NULL DEFAULT '',
    phone                  TEXT NOT NULL DEFAULT '',
    emergency_first_name   TEXT NOT NULL DEFAULT '',
    emergency_last_name    TEXT NOT NULL DEFAULT '',
    emergency_relationship TEXT NOT NULL DEFAULT '',
    emergency_phone        TEXT NOT NULL DEFAULT '',
    legal_name             TEXT NOT NULL DEFAULT '',
    signature_name         TEXT NOT NULL DEFAULT '',
    signer_is_guardian     BOOLEAN NOT NULL DEFAULT false,
    guardian_name          TEXT NOT NULL DEFAULT '',
    guardian_relationship  TEXT NOT NULL DEFAULT '',
    guardian_email         TEXT NOT NULL DEFAULT '',
    electronic_consent     BOOLEAN NOT NULL DEFAULT false,
    signed_at              TIMESTAMPTZ,

    -- 'pending' awaits a decision; 'denied' is final; 'approved' has a live
    -- setup link outstanding; 'completed' has become a real account.
    status            TEXT NOT NULL DEFAULT 'pending'
                      CHECK (status IN ('pending', 'approved', 'denied', 'completed')),
    decided_by        TEXT        REFERENCES users(id) ON DELETE SET NULL,
    decided_by_name   TEXT        NOT NULL DEFAULT '',
    decision_note     TEXT        NOT NULL DEFAULT '',
    decided_at        TIMESTAMPTZ,

    -- The official address an approving admin may issue. Empty means they keep
    -- the address they applied with.
    assigned_email    TEXT NOT NULL DEFAULT '',
    -- SHA-256 of the setup token; the raw token only ever exists in the emailed
    -- link, mirroring `password_reset_tokens`.
    setup_token_hash  TEXT,
    setup_expires_at  TIMESTAMPTZ,
    completed_user_id TEXT REFERENCES users(id) ON DELETE SET NULL
);

-- The hot read is the admin's pending queue and its count badge.
CREATE INDEX volunteer_applicants_pending_idx
    ON volunteer_applicants (agreed_at) WHERE status = 'pending';

-- Token lookup on the setup page.
CREATE INDEX volunteer_applicants_setup_token_idx
    ON volunteer_applicants (setup_token_hash) WHERE setup_token_hash IS NOT NULL;

-- One live application per address, case-insensitively, matching how `users`
-- treats email. A declined applicant may apply again; a completed one now has
-- an account, and `users.email` is what stops a duplicate from there.
CREATE UNIQUE INDEX volunteer_applicants_live_email_idx
    ON volunteer_applicants (lower(email)) WHERE status IN ('pending', 'approved');
