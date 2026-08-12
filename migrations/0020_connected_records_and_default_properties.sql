-- Default CRM properties and the missing account/person/case connections.

CREATE TABLE organization_properties (
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    ord             INTEGER NOT NULL,
    key             TEXT NOT NULL,
    value           TEXT NOT NULL DEFAULT '',
    section         TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (organization_id, ord),
    CONSTRAINT organization_properties_key_check CHECK (btrim(key) <> '')
);

-- Preserve older manually created rows while tightening the standing rule.
UPDATE contacts SET types = ARRAY['other'] WHERE cardinality(types) = 0;
ALTER TABLE contacts
    ADD CONSTRAINT contacts_types_nonempty_check CHECK (cardinality(types) > 0);

-- Repair account/person gaps without replacing an explicit existing link.
INSERT INTO contacts
    (id, first_name, last_name, email, phone, address, user_id, types, source)
SELECT
    'ct-' || nextval('app_id_seq'),
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
WHERE btrim(u.last_name) <> ''
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.user_id = u.id)
ON CONFLICT DO NOTHING;

-- Append only defaults whose normalized section/name pair is absent. Existing
-- rows keep both their values and their display order.
WITH defaults(section, key, default_ord) AS (
    VALUES
        ('Communication', 'Preferred contact method', 0),
        ('Communication', 'Best time to reach', 1),
        ('Communication', 'Preferred language', 2),
        ('Relationship', 'Relationship status', 3),
        ('Relationship', 'Last contacted on', 4),
        ('Relationship', 'Next follow-up on', 5)
), missing AS (
    SELECT c.id AS contact_id, d.section, d.key, d.default_ord,
           coalesce((SELECT max(cp.ord) + 1 FROM contact_properties cp
                     WHERE cp.contact_id = c.id), 0) AS start_ord
    FROM contacts c
    CROSS JOIN defaults d
    WHERE NOT EXISTS (
        SELECT 1 FROM contact_properties cp
        WHERE cp.contact_id = c.id
          AND lower(regexp_replace(btrim(cp.section), '\s+', ' ', 'g')) =
              lower(regexp_replace(btrim(d.section), '\s+', ' ', 'g'))
          AND lower(regexp_replace(btrim(cp.key), '\s+', ' ', 'g')) =
              lower(regexp_replace(btrim(d.key), '\s+', ' ', 'g'))
    )
)
INSERT INTO contact_properties (contact_id, ord, key, value, section)
SELECT contact_id,
       (start_ord + row_number() OVER (PARTITION BY contact_id ORDER BY default_ord) - 1)::integer,
       key, '', section
FROM missing;

WITH defaults(section, key, default_ord) AS (
    VALUES
        ('Relationship', 'Relationship status', 0),
        ('Relationship', 'Preferred contact method', 1),
        ('Relationship', 'Last contacted on', 2),
        ('Relationship', 'Next follow-up on', 3)
)
INSERT INTO organization_properties (organization_id, ord, key, value, section)
SELECT o.id, d.default_ord, d.key, '', d.section
FROM organizations o
CROSS JOIN defaults d;

-- The client account that owns a case is also the obvious CRM person on it.
-- Preserve every existing primary and every existing person/role choice.
INSERT INTO case_contacts
    (id, case_id, contact_id, role, note, is_primary, added_by)
SELECT
    'cc-' || nextval('app_id_seq'),
    ca.id,
    ct.id,
    'client',
    '',
    NOT EXISTS (SELECT 1 FROM case_contacts primary_cc
                WHERE primary_cc.case_id = ca.id AND primary_cc.is_primary),
    'Migration'
FROM cases ca
JOIN users u ON u.id = ca.owner_id AND u.role = 'client'
JOIN contacts ct ON ct.user_id = u.id AND NOT ct.archived
WHERE NOT EXISTS (
    SELECT 1 FROM case_contacts cc
    WHERE cc.case_id = ca.id AND cc.contact_id = ct.id AND cc.role = 'client'
)
ON CONFLICT DO NOTHING;
