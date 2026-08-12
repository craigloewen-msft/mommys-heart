-- CRM production-readiness invariants: stable actor attribution, first-class
-- funding audit scope, linked-account conflict visibility, and evidence scanning.

-- REQ-CRM-048: legacy rows keep their display snapshot; new CRM writes also
-- record the immutable account id that authenticated the mutation.
ALTER TABLE audit_log
    ADD COLUMN actor_user_id TEXT REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX audit_log_actor_user_idx ON audit_log(actor_user_id, created_at DESC);

ALTER TABLE funding
    ADD COLUMN recorded_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN voided_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX funding_recorded_by_user_idx ON funding(recorded_by_user_id);
CREATE INDEX funding_voided_by_user_idx ON funding(voided_by_user_id);

-- Preserve account/contact disagreements instead of overwriting either side.
-- The application projects account-owned identity fields while the link exists.
CREATE TABLE contact_account_conflicts (
    contact_id      TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    field           TEXT NOT NULL,
    contact_value   TEXT NOT NULL,
    account_value   TEXT NOT NULL,
    detected_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (contact_id, field),
    CONSTRAINT contact_account_conflicts_field_check CHECK (field IN (
        'first_name', 'last_name', 'email', 'phone', 'address'
    ))
);

INSERT INTO contact_account_conflicts (contact_id, field, contact_value, account_value)
SELECT c.id, differences.field, differences.contact_value, differences.account_value
FROM contacts c
JOIN users u ON u.id = c.user_id
CROSS JOIN LATERAL (VALUES
    ('first_name', c.first_name, u.first_name),
    ('last_name', c.last_name, u.last_name),
    ('email', c.email, u.email),
    ('phone', c.phone, u.phone),
    ('address', c.address, u.home_address)
) AS differences(field, contact_value, account_value)
WHERE differences.contact_value <> differences.account_value;

-- REQ-SEC-005: existing files are grandfathered as clean; every new upload is
-- scanned before its blob path becomes downloadable.
ALTER TABLE evidence
    ADD COLUMN scan_status TEXT NOT NULL DEFAULT 'clean',
    ADD COLUMN scanned_at TIMESTAMPTZ,
    ADD COLUMN scan_detail TEXT NOT NULL DEFAULT '',
    ADD CONSTRAINT evidence_scan_status_check
        CHECK (scan_status IN ('none', 'pending', 'clean', 'rejected')),
    ADD CONSTRAINT evidence_clean_file_check
        CHECK (blob_path = '' OR scan_status = 'clean');

UPDATE evidence
SET scanned_at = now(),
    scan_detail = CASE WHEN blob_path = '' THEN '' ELSE 'Legacy file accepted before scanner gate' END,
    scan_status = CASE WHEN blob_path = '' THEN 'none' ELSE 'clean' END;

CREATE TABLE evidence_scan_log (
    id              BIGSERIAL PRIMARY KEY,
    evidence_id     TEXT NOT NULL,
    case_id         TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    actor_user_id   TEXT REFERENCES users(id) ON DELETE SET NULL,
    actor            TEXT NOT NULL,
    sha256           TEXT NOT NULL,
    status           TEXT NOT NULL CHECK (status IN ('clean', 'rejected', 'error')),
    detail           TEXT NOT NULL DEFAULT '',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX evidence_scan_log_case_idx ON evidence_scan_log(case_id, created_at DESC);
CREATE INDEX evidence_scan_log_evidence_idx ON evidence_scan_log(evidence_id, created_at DESC);
