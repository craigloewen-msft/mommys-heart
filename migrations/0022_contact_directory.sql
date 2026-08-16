-- The outreach contact directory: a reusable classification taxonomy and a
-- communication log layered over the CRM `contacts` records from 0019.
--
-- Classifications are rows rather than enum values so the organization can
-- extend the taxonomy without a migration.

-- Contacts gained a public web presence field; organizations already had one.
ALTER TABLE contacts ADD COLUMN website TEXT NOT NULL DEFAULT '';

CREATE TABLE contact_categories (
    id         TEXT PRIMARY KEY,
    name       TEXT        NOT NULL CHECK (btrim(name) <> ''),
    parent_id  TEXT        REFERENCES contact_categories(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX contact_categories_root_name_idx
    ON contact_categories (lower(name))
    WHERE parent_id IS NULL;
CREATE UNIQUE INDEX contact_categories_child_name_idx
    ON contact_categories (parent_id, lower(name))
    WHERE parent_id IS NOT NULL;

CREATE TABLE contact_category_assignments (
    contact_id  TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES contact_categories(id) ON DELETE RESTRICT,
    PRIMARY KEY (contact_id, category_id)
);

CREATE INDEX contact_category_assignments_category_idx
    ON contact_category_assignments (category_id, contact_id);

CREATE TABLE contact_communications (
    id          TEXT PRIMARY KEY,
    contact_id  TEXT        NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    kind        TEXT        NOT NULL CHECK (kind IN (
                    'outreach', 'conversation', 'referral', 'follow_up',
                    'relationship', 'note'
                )),
    body        TEXT        NOT NULL CHECK (btrim(body) <> ''),
    author_id   TEXT        REFERENCES users(id) ON DELETE SET NULL,
    author_name TEXT        NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX contact_communications_contact_idx
    ON contact_communications (contact_id, occurred_at DESC);
