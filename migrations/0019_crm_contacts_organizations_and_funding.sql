-- The CRM layer: people the organization knows, the organizations they belong
-- to, custom properties on a person, who is involved in a case, and the money
-- side (grants and the funding received against them).
--
-- The central split: a `users` row is an ACCOUNT that can sign in; a `contacts`
-- row is a PERSON the organization has a relationship with. Most contacts never
-- get an account (donors, funder program officers, partner caseworkers,
-- opposing attorneys, emergency contacts). A contact may link to at most one
-- user, but `users` remains the source of truth for identity and authentication:
-- nothing here stores a password, a role, or a second login path.

-- ---------------------------------------------------------------------------
-- Organizations
-- ---------------------------------------------------------------------------
CREATE TABLE organizations (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL,
    website     TEXT NOT NULL DEFAULT '',
    phone       TEXT NOT NULL DEFAULT '',
    email       TEXT NOT NULL DEFAULT '',
    address     TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    archived    BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq         BIGSERIAL,
    CONSTRAINT organizations_name_check CHECK (btrim(name) <> ''),
    CONSTRAINT organizations_kind_check CHECK (kind IN (
        'funder', 'partner', 'service_provider', 'government', 'court',
        'employer', 'other'
    ))
);

-- Two active organizations may not share a name; archived rows are exempt so a
-- name can be reused after an organization is retired.
CREATE UNIQUE INDEX organizations_active_name_key
    ON organizations (lower(btrim(name))) WHERE NOT archived;
CREATE INDEX organizations_kind_idx ON organizations (kind, archived);

-- ---------------------------------------------------------------------------
-- Contacts
-- ---------------------------------------------------------------------------
CREATE TABLE contacts (
    id              TEXT PRIMARY KEY,
    first_name      TEXT NOT NULL DEFAULT '',
    last_name       TEXT NOT NULL DEFAULT '',
    preferred_name  TEXT NOT NULL DEFAULT '',
    email           TEXT NOT NULL DEFAULT '',
    phone           TEXT NOT NULL DEFAULT '',
    mobile          TEXT NOT NULL DEFAULT '',
    address         TEXT NOT NULL DEFAULT '',
    job_title       TEXT NOT NULL DEFAULT '',
    organization_id TEXT REFERENCES organizations(id) ON DELETE RESTRICT,
    -- The account this person signs in with, when they have one at all.
    -- ON DELETE SET NULL: removing an account must not erase the person record.
    user_id         TEXT REFERENCES users(id) ON DELETE SET NULL,
    types           TEXT[] NOT NULL DEFAULT '{}',
    source          TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    do_not_contact  BOOLEAN NOT NULL DEFAULT false,
    archived        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq             BIGSERIAL,
    -- A row with neither a surname nor an employer has nothing to show in a
    -- directory, so at least one must be present.
    CONSTRAINT contacts_named_check
        CHECK (btrim(last_name) <> '' OR organization_id IS NOT NULL),
    CONSTRAINT contacts_types_check CHECK (types <@ ARRAY[
        'client', 'volunteer', 'staff', 'donor', 'funder_contact', 'partner',
        'service_provider', 'attorney', 'court_professional',
        'government_agency', 'emergency_contact', 'board_member', 'other'
    ]::text[])
);

-- One contact per account, at most. Partial so the many account-less contacts
-- do not collide with each other on NULL.
CREATE UNIQUE INDEX contacts_user_id_key ON contacts (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX contacts_organization_idx ON contacts (organization_id);
CREATE INDEX contacts_archived_seq_idx ON contacts (archived, seq DESC);
CREATE INDEX contacts_types_idx ON contacts USING GIN (types);
-- Directory keyword search is a substring (ILIKE) match, which no btree or
-- tsvector index can serve; a trigram index would need the pg_trgm extension.
-- Left unindexed on purpose rather than adding one the planner would ignore.

-- ---------------------------------------------------------------------------
-- Contact properties
--
-- The same shape as `case_properties` (see 0001/0004): an ordered key/value list
-- grouped under a free-text section heading, rewritten in place.
--
-- Deliberately WITHOUT a `visibility` column. On a case, shared vs
-- volunteer_only answers "may the client see this?"; clients cannot see contacts
-- at all, so reusing that vocabulary here would give it a second, weaker
-- meaning. Field-level sensitivity is future work, not this column.
-- ---------------------------------------------------------------------------
CREATE TABLE contact_properties (
    contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    ord        INTEGER NOT NULL,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL DEFAULT '',
    section    TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (contact_id, ord),
    CONSTRAINT contact_properties_key_check CHECK (btrim(key) <> '')
);

-- ---------------------------------------------------------------------------
-- Case contacts: who is involved in a case, and in what role
-- ---------------------------------------------------------------------------
CREATE TABLE case_contacts (
    id         TEXT PRIMARY KEY,
    case_id    TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE RESTRICT,
    role       TEXT NOT NULL,
    note       TEXT NOT NULL DEFAULT '',
    is_primary BOOLEAN NOT NULL DEFAULT false,
    added_by   TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq        BIGSERIAL,
    CONSTRAINT case_contacts_role_check CHECK (role IN (
        'client', 'household_member', 'emergency_contact', 'attorney',
        'opposing_party', 'caseworker', 'provider_contact',
        'court_professional', 'other'
    )),
    -- The same person cannot hold the same role on a case twice.
    CONSTRAINT case_contacts_unique UNIQUE (case_id, contact_id, role)
);

-- At most one primary contact per case.
CREATE UNIQUE INDEX case_contacts_one_primary_idx
    ON case_contacts (case_id) WHERE is_primary;
CREATE INDEX case_contacts_case_idx ON case_contacts (case_id, seq DESC);
CREATE INDEX case_contacts_contact_idx ON case_contacts (contact_id);

-- ---------------------------------------------------------------------------
-- Grants: the `id`/`name` stub from 0001 becomes a real award record.
--
-- Additive only, so existing rows survive: every new column is nullable or
-- defaulted. Money is integer minor units (cents) throughout — never floating
-- point, which cannot represent a currency amount exactly.
-- ---------------------------------------------------------------------------
ALTER TABLE grants
    ADD COLUMN funder_organization_id TEXT REFERENCES organizations(id) ON DELETE RESTRICT,
    ADD COLUMN program_officer_contact_id TEXT REFERENCES contacts(id) ON DELETE RESTRICT,
    ADD COLUMN status TEXT NOT NULL DEFAULT 'prospect',
    ADD COLUMN amount_requested_cents BIGINT,
    ADD COLUMN amount_awarded_cents BIGINT,
    ADD COLUMN application_date DATE,
    ADD COLUMN decision_date DATE,
    ADD COLUMN period_start DATE,
    ADD COLUMN period_end DATE,
    ADD COLUMN purpose TEXT NOT NULL DEFAULT '',
    ADD COLUMN reporting_cadence TEXT NOT NULL DEFAULT 'none',
    ADD COLUMN notes TEXT NOT NULL DEFAULT '',
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN seq BIGSERIAL;

ALTER TABLE grants
    ADD CONSTRAINT grants_status_check CHECK (status IN (
        'prospect', 'applied', 'awarded', 'active', 'reporting', 'closed', 'declined'
    )),
    ADD CONSTRAINT grants_reporting_cadence_check CHECK (reporting_cadence IN (
        'none', 'monthly', 'quarterly', 'semiannual', 'annual', 'final_only'
    )),
    ADD CONSTRAINT grants_amounts_check CHECK (
        (amount_requested_cents IS NULL OR amount_requested_cents >= 0)
        AND (amount_awarded_cents IS NULL OR amount_awarded_cents >= 0)
    ),
    ADD CONSTRAINT grants_period_order_check
        CHECK (period_start IS NULL OR period_end IS NULL OR period_end >= period_start),
    -- An awarded grant must say how much and for how long; a decided one must
    -- say when. Enforced here so the rule holds even outside the app.
    ADD CONSTRAINT grants_awarded_requires_terms_check CHECK (
        status NOT IN ('awarded', 'active', 'reporting', 'closed')
        OR (amount_awarded_cents IS NOT NULL
            AND period_start IS NOT NULL
            AND period_end IS NOT NULL)
    ),
    ADD CONSTRAINT grants_decided_requires_date_check CHECK (
        status NOT IN ('closed', 'declined') OR decision_date IS NOT NULL
    );

CREATE INDEX grants_status_idx ON grants (status, seq DESC);
CREATE INDEX grants_funder_idx ON grants (funder_organization_id);

-- ---------------------------------------------------------------------------
-- Funding: money received, against a grant or as a standalone gift
-- ---------------------------------------------------------------------------
CREATE TABLE funding (
    id                     TEXT PRIMARY KEY,
    kind                   TEXT NOT NULL,
    amount_cents           BIGINT NOT NULL,
    received_on            DATE NOT NULL,
    grant_id               TEXT REFERENCES grants(id) ON DELETE RESTRICT,
    source_organization_id TEXT REFERENCES organizations(id) ON DELETE RESTRICT,
    source_contact_id      TEXT REFERENCES contacts(id) ON DELETE RESTRICT,
    reference              TEXT NOT NULL DEFAULT '',
    notes                  TEXT NOT NULL DEFAULT '',
    -- A mistaken record is voided with a reason, never deleted: it leaves the
    -- rollups but stays visible and audited (REQ-AUD-004).
    voided                 BOOLEAN NOT NULL DEFAULT false,
    void_reason            TEXT NOT NULL DEFAULT '',
    voided_by              TEXT NOT NULL DEFAULT '',
    voided_at              TIMESTAMPTZ,
    recorded_by            TEXT NOT NULL DEFAULT '',
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq                    BIGSERIAL,
    CONSTRAINT funding_kind_check CHECK (kind IN (
        'grant_payment', 'donation', 'in_kind', 'other'
    )),
    CONSTRAINT funding_amount_check CHECK (amount_cents > 0),
    -- A grant payment must name its grant; a gift must name where it came from.
    CONSTRAINT funding_grant_payment_check
        CHECK (kind <> 'grant_payment' OR grant_id IS NOT NULL),
    CONSTRAINT funding_donation_source_check CHECK (
        kind <> 'donation'
        OR source_organization_id IS NOT NULL
        OR source_contact_id IS NOT NULL
    ),
    CONSTRAINT funding_void_reason_check
        CHECK (NOT voided OR (btrim(void_reason) <> '' AND voided_at IS NOT NULL))
);

CREATE INDEX funding_grant_idx ON funding (grant_id) WHERE NOT voided;
CREATE INDEX funding_received_idx ON funding (received_on DESC, seq DESC);
CREATE INDEX funding_org_idx ON funding (source_organization_id);
CREATE INDEX funding_contact_idx ON funding (source_contact_id);

-- Funding rows are corrected by voiding, not deleting.
CREATE OR REPLACE FUNCTION prevent_funding_delete()
RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'funding records are voided, not deleted';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER funding_prevent_delete
BEFORE DELETE ON funding
FOR EACH ROW EXECUTE FUNCTION prevent_funding_delete();

-- ---------------------------------------------------------------------------
-- Backfill: one contact per existing account, so the directory is populated and
-- current staff and clients are manageable from the moment this ships.
--
-- `id` mirrors the user id (`u-abc` -> `ct-u-abc`) rather than drawing from the
-- shared sequence, which keeps the backfill deterministic and re-runnable.
-- ---------------------------------------------------------------------------
INSERT INTO contacts (id, first_name, last_name, email, phone, address, user_id, types, source)
SELECT
    'ct-' || u.id,
    u.first_name,
    u.last_name,
    u.email,
    u.phone,
    u.home_address,
    u.id,
    CASE u.role
        WHEN 'client' THEN ARRAY['client']
        WHEN 'volunteer' THEN ARRAY['volunteer']
        ELSE ARRAY['staff']
    END,
    'Account backfill'
FROM users u
-- A user with no surname and no employer would violate contacts_named_check.
WHERE btrim(u.last_name) <> ''
ON CONFLICT DO NOTHING;
