-- Carry a case intake and its already-staged signed agreement through the
-- existing email-verification challenge. The user, case, properties, evidence
-- row, and assignments are materialized together only after the code matches.
ALTER TABLE pending_registrations
    ADD COLUMN create_case                   BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN case_id                       TEXT NOT NULL DEFAULT '',
    ADD COLUMN case_name                     TEXT NOT NULL DEFAULT '',
    ADD COLUMN intake_json                   TEXT NOT NULL DEFAULT '{}',
    ADD COLUMN agreement_evidence_id          TEXT NOT NULL DEFAULT '',
    ADD COLUMN agreement_original_filename   TEXT NOT NULL DEFAULT '',
    ADD COLUMN agreement_content_type        TEXT NOT NULL DEFAULT '',
    ADD COLUMN agreement_size_bytes          BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN agreement_sha256              TEXT NOT NULL DEFAULT '',
    ADD COLUMN agreement_blob_path           TEXT NOT NULL DEFAULT '',
    ADD CONSTRAINT pending_case_signup_complete CHECK (
        NOT create_case OR (
            case_id <> ''
            AND case_name <> ''
            AND agreement_evidence_id <> ''
            AND agreement_original_filename <> ''
            AND agreement_content_type <> ''
            AND agreement_size_bytes > 0
            AND agreement_sha256 <> ''
            AND agreement_blob_path <> ''
        )
    );