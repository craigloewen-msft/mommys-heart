-- Consent moves from a document to a record. A prospective client used to
-- download the service agreement, sign it, and upload the .docx back; the signed
-- file was staged in object storage and filed as evidence once the email code
-- verified. Now they read a Terms and Conditions page and accept it before they
-- ever reach the case form, so what the database carries is the fact of
-- acceptance -- who, which version, and when -- not a blob.

-- The staged-agreement columns described a file that no longer exists. Pending
-- registrations live for ten minutes, so there is nothing worth migrating out of
-- them; any row still holding one is abandoned by the time this runs.
ALTER TABLE pending_registrations
    DROP CONSTRAINT IF EXISTS pending_case_signup_complete,
    DROP COLUMN agreement_evidence_id,
    DROP COLUMN agreement_original_filename,
    DROP COLUMN agreement_content_type,
    DROP COLUMN agreement_size_bytes,
    DROP COLUMN agreement_sha256,
    DROP COLUMN agreement_blob_path,
    ADD COLUMN terms_version TEXT NOT NULL DEFAULT '',
    ADD CONSTRAINT pending_case_signup_complete CHECK (
        NOT create_case OR (
            case_id <> ''
            AND case_name <> ''
            AND terms_version <> ''
        )
    );

-- One row per acceptance, kept even if the terms are later revised: the version
-- string records which wording the client agreed to, so a bump never rewrites
-- history. `case_id` is nullable because acceptance is the person's, and the
-- case it was given for may be deleted without erasing that they consented.
CREATE TABLE terms_acceptances (
    id            TEXT PRIMARY KEY,
    user_id       TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    case_id       TEXT        REFERENCES cases(id) ON DELETE SET NULL,
    terms_version TEXT        NOT NULL,
    accepted_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX terms_acceptances_user_idx ON terms_acceptances (user_id);
CREATE INDEX terms_acceptances_case_idx ON terms_acceptances (case_id);
