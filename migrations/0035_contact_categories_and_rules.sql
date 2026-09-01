-- Contact classification, cleaned up, plus the rules layer that drives
-- suggested categories and properties.
--
-- Three things happen here:
--   1. The taxonomy gains an explicit order and one naming convention, and
--      becomes owned by the database rather than re-seeded from Rust presets
--      at every boot (so a category the organization deletes stays deleted).
--   2. `Vendor` joins the contact types.
--   3. Rules arrive: "when a contact is X, offer these grouped categories and
--      these property fields". Rules only decide what the editor *offers* --
--      what gets saved is still ordinary assignment and property rows.

-- ---------------------------------------------------------------------------
-- 1. The taxonomy
-- ---------------------------------------------------------------------------

-- Categories had no order at all, so a list could only ever be alphabetical.
-- The organization's own lists are not (locations read New York, Florida,
-- Hamptons), so display order has to be storable.
ALTER TABLE contact_categories ADD COLUMN ord INT NOT NULL DEFAULT 0;

-- " / " is now reserved for the parent/child separator `ContactCategory::label`
-- builds. Names that used it to mean "or" are rewritten with "and", so a badge
-- reading "Medical and Mental Health / Therapists" has exactly one separator.
-- Capitalization is Title Case at both levels; roots were already Title Case
-- and children were sentence case.
UPDATE contact_categories AS c
SET name = v.name, ord = v.ord
FROM (VALUES
    ('cc-legal',                        'Legal',                          0),
    ('cc-law-firms',                    'Law Firms',                      0),
    ('cc-individual-attorneys',         'Individual Attorneys',           1),
    ('cc-family-law',                   'Family Law',                     2),
    ('cc-domestic-violence-legal',      'Domestic Violence',              3),
    ('cc-matrimonial-law',              'Matrimonial Law',                4),
    ('cc-appellate-law',                'Appellate Law',                  5),
    ('cc-law-schools',                  'Law Schools',                    6),
    ('cc-law-school-clinics',           'Law School Clinics',             7),
    ('cc-legal-services-organizations', 'Legal Services Organizations',   8),

    ('cc-social-services',              'Social Services',                1),
    ('cc-housing',                      'Housing',                        0),
    ('cc-public-benefits',              'Public Benefits',                1),
    ('cc-domestic-violence-services',   'Domestic Violence Services',     2),
    ('cc-mental-health-services',       'Mental Health',                  3),
    ('cc-child-family-services',        'Child and Family Services',      4),
    ('cc-food-assistance',              'Food Assistance',                5),
    ('cc-financial-assistance',         'Financial Assistance',           6),
    ('cc-employment-career-services',   'Employment and Career Services', 7),
    ('cc-community-based-organizations','Community-Based Organizations',  8),

    ('cc-education',                    'Education',                      2),
    ('cc-colleges-universities',        'Colleges and Universities',      0),
    ('cc-graduate-schools',             'Graduate Schools',               1),
    ('cc-social-work-schools',          'Social Work Schools',            2),
    ('cc-education-law-schools',        'Law Schools',                    3),
    ('cc-field-placement-offices',      'Internship and Field Placement Offices', 4),
    ('cc-career-services',              'Career Services',                5),
    ('cc-alumni-associations',          'Alumni Associations',            6),
    ('cc-faculty-professors',           'Faculty and Professors',         7),

    ('cc-medical-mental-health',        'Medical and Mental Health',      3),
    ('cc-therapists',                   'Therapists',                     0),
    ('cc-social-workers',               'Social Workers',                 1),
    ('cc-psychologists',                'Psychologists',                  2),
    ('cc-psychiatrists',                'Psychiatrists',                  3),
    ('cc-physicians',                   'Physicians',                     4),
    ('cc-clinics-hospitals',            'Clinics and Hospitals',          5),

    ('cc-government-public-affairs',    'Government and Public Affairs',  4),
    ('cc-public-officials',             'Public Officials',               0),
    ('cc-retired-public-officials',     'Retired Public Officials',       1),
    ('cc-government-agencies',          'Government Agencies',            2),
    ('cc-legislative-offices',          'Legislative Offices',            3),
    ('cc-policy-advocacy-organizations','Policy and Advocacy Organizations', 4),

    ('cc-media-entertainment',          'Media and Entertainment',        5),
    ('cc-actors',                       'Actors',                         0),
    ('cc-musicians',                    'Musicians',                      1),
    ('cc-tv-hosts',                     'TV Hosts',                       2),
    ('cc-newscasters',                  'Newscasters',                    3),
    ('cc-reporters-journalists',        'Reporters and Journalists',      4),
    ('cc-producers',                    'Producers',                      5),
    ('cc-media-organizations',          'Media Organizations',            6),

    ('cc-philanthropy-fundraising',     'Philanthropy and Fundraising',   6),
    ('cc-philanthropists',              'Philanthropists',                0),
    ('cc-foundations',                  'Foundations',                    1),
    ('cc-corporate-sponsors',           'Corporate Sponsors',             2),
    ('cc-donors',                       'Donors',                         3),
    ('cc-prospective-donors',           'Prospective Donors',             4),
    ('cc-funders-grantmakers',          'Funders and Grantmaking Organizations', 5),

    ('cc-business-corporate',           'Business and Corporate',         7),
    ('cc-companies',                    'Companies',                      0),
    ('cc-corporate-partners',           'Corporate Partners',             1),
    ('cc-professional-services',        'Professional Services',          2),
    ('cc-prospective-sponsors',         'Prospective Sponsors',           3),

    ('cc-nonprofit-community',          'Nonprofit and Community',        8),
    ('cc-nonprofit-organizations',      'Nonprofit Organizations',        0),
    ('cc-advocacy-organizations',       'Advocacy Organizations',         1),
    ('cc-community-organizations',      'Community Organizations',        2),
    ('cc-referral-partners',            'Referral Partners',              3),
    ('cc-prospective-partners',         'Prospective Partners',           4)
) AS v(id, name, ord)
WHERE c.id = v.id;

-- "Business and Corporate / Vendors" is superseded by the Vendor root below.
-- Guarded rather than unconditional: if anyone has classified a contact under
-- it since this was written, the row stays and is merged by hand instead.
DELETE FROM contact_categories
WHERE id = 'cc-vendors'
  AND NOT EXISTS (
      SELECT 1 FROM contact_category_assignments WHERE category_id = 'cc-vendors'
  );

-- The vendor taxonomy. Ordinary rows: renameable, reorderable, deletable.
INSERT INTO contact_categories (id, name, parent_id, ord) VALUES
    ('cc-vendor', 'Vendor', NULL, 9)
ON CONFLICT DO NOTHING;

INSERT INTO contact_categories (id, name, parent_id, ord) VALUES
    ('cc-vendor-new-york',       'New York Vendors',      'cc-vendor', 0),
    ('cc-vendor-florida',        'Florida Vendors',       'cc-vendor', 1),
    ('cc-vendor-hamptons',       'Hamptons Vendors',      'cc-vendor', 2),
    ('cc-vendor-restaurants',    'Restaurants',           'cc-vendor', 3),
    ('cc-vendor-caterers',       'Caterers',              'cc-vendor', 4),
    ('cc-vendor-bakeries',       'Bakeries and Desserts', 'cc-vendor', 5),
    ('cc-vendor-wine-liquor',    'Wine and Liquor',       'cc-vendor', 6),
    ('cc-vendor-event-venues',   'Event Venues',          'cc-vendor', 7),
    ('cc-vendor-hotels',         'Hotels and Hospitality','cc-vendor', 8),
    ('cc-vendor-party-supplies', 'Party Supplies',        'cc-vendor', 9),
    ('cc-vendor-event-rentals',  'Event Rentals',         'cc-vendor', 10),
    ('cc-vendor-event-planners', 'Event Planners and Coordinators', 'cc-vendor', 11),
    ('cc-vendor-florists',       'Florists',              'cc-vendor', 12),
    ('cc-vendor-decor-design',   'Décor and Design',      'cc-vendor', 13),
    ('cc-vendor-entertainment',  'Entertainment and Performers',    'cc-vendor', 14),
    ('cc-vendor-djs-musicians',  'DJs and Musicians',     'cc-vendor', 15),
    ('cc-vendor-photo-video',    'Photography and Videography',     'cc-vendor', 16),
    ('cc-vendor-av-production',  'Audio, Visual, Lighting, and Production', 'cc-vendor', 17),
    ('cc-vendor-printing',       'Printing, Invitations, and Signage',      'cc-vendor', 18),
    ('cc-vendor-marketing',      'Marketing and Promotional Products',      'cc-vendor', 19),
    ('cc-vendor-gift-bags',      'Gift Bags and Product Donations',         'cc-vendor', 20),
    ('cc-vendor-auction',        'Auction and Raffle Donations',            'cc-vendor', 21),
    ('cc-vendor-fashion-beauty', 'Fashion, Beauty, and Wellness',           'cc-vendor', 22),
    ('cc-vendor-transportation', 'Transportation',        'cc-vendor', 23),
    ('cc-vendor-staffing',       'Staffing and Security', 'cc-vendor', 24),
    ('cc-vendor-other',          'Other',                 'cc-vendor', 25)
ON CONFLICT DO NOTHING;

-- ---------------------------------------------------------------------------
-- 2. Vendor as a contact type
-- ---------------------------------------------------------------------------

ALTER TABLE contacts DROP CONSTRAINT contacts_types_check;
ALTER TABLE contacts ADD CONSTRAINT contacts_types_check CHECK (types <@ ARRAY[
    'client', 'volunteer', 'staff', 'donor', 'funder_contact', 'partner',
    'service_provider', 'attorney', 'court_professional', 'government_agency',
    'emergency_contact', 'board_member', 'vendor', 'other'
]::text[]);

-- ---------------------------------------------------------------------------
-- 3. Rules
-- ---------------------------------------------------------------------------

-- A rule fires when a contact carries its trigger, and offers the groups and
-- fields below. `trigger_value` is a ContactType slug or a category id
-- depending on `trigger_kind`; it is not a foreign key because it addresses two
-- different vocabularies, so the application resolves it.
CREATE TABLE contact_rules (
    id            TEXT PRIMARY KEY,
    name          TEXT        NOT NULL CHECK (btrim(name) <> ''),
    description   TEXT        NOT NULL DEFAULT '',
    trigger_kind  TEXT        NOT NULL CHECK (trigger_kind IN ('contact_type', 'category')),
    trigger_value TEXT        NOT NULL CHECK (btrim(trigger_value) <> ''),
    active        BOOLEAN     NOT NULL DEFAULT true,
    ord           INT         NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX contact_rules_trigger_idx ON contact_rules (trigger_kind, trigger_value)
    WHERE active;

-- One labelled block of category choices, with how many may be picked:
-- max_choices = 1 is a single-select, NULL is unlimited.
CREATE TABLE contact_rule_groups (
    id          TEXT PRIMARY KEY,
    rule_id     TEXT NOT NULL REFERENCES contact_rules(id) ON DELETE CASCADE,
    label       TEXT NOT NULL CHECK (btrim(label) <> ''),
    help_text   TEXT NOT NULL DEFAULT '',
    min_choices INT  NOT NULL DEFAULT 0,
    max_choices INT,
    ord         INT  NOT NULL DEFAULT 0,
    CONSTRAINT contact_rule_groups_choices_check
        CHECK (min_choices >= 0 AND (max_choices IS NULL OR max_choices >= 1))
);

CREATE INDEX contact_rule_groups_rule_idx ON contact_rule_groups (rule_id, ord);

CREATE TABLE contact_rule_group_options (
    group_id    TEXT NOT NULL REFERENCES contact_rule_groups(id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES contact_categories(id) ON DELETE CASCADE,
    ord         INT  NOT NULL DEFAULT 0,
    PRIMARY KEY (group_id, category_id)
);

-- Blank property rows a matching contact gets, named the same way every other
-- property is: a section heading plus a key.
CREATE TABLE contact_rule_fields (
    id            TEXT   PRIMARY KEY,
    rule_id       TEXT   NOT NULL REFERENCES contact_rules(id) ON DELETE CASCADE,
    section       TEXT   NOT NULL DEFAULT '',
    key           TEXT   NOT NULL CHECK (btrim(key) <> ''),
    default_value TEXT   NOT NULL DEFAULT '',
    suggestions   TEXT[] NOT NULL DEFAULT '{}',
    ord           INT    NOT NULL DEFAULT 0
);

CREATE INDEX contact_rule_fields_rule_idx ON contact_rule_fields (rule_id, ord);

-- The vendor rule: pick one location, pick any number of vendor types.
INSERT INTO contact_rules (id, name, description, trigger_kind, trigger_value, ord) VALUES
    ('cr-vendor', 'Vendor',
     'Offers the vendor location and vendor type lists when a contact is marked as a vendor.',
     'contact_type', 'vendor', 0);

INSERT INTO contact_rule_groups (id, rule_id, label, help_text, min_choices, max_choices, ord) VALUES
    ('crg-vendor-location', 'cr-vendor', 'Location',
     'Where this vendor operates. One only.', 1, 1, 0),
    ('crg-vendor-type', 'cr-vendor', 'Vendor type',
     'What this vendor provides. Choose as many as apply.', 1, NULL, 1);

INSERT INTO contact_rule_group_options (group_id, category_id, ord)
SELECT 'crg-vendor-location', id, ord
FROM contact_categories
WHERE id IN ('cc-vendor-new-york', 'cc-vendor-florida', 'cc-vendor-hamptons');

INSERT INTO contact_rule_group_options (group_id, category_id, ord)
SELECT 'crg-vendor-type', id, ord
FROM contact_categories
WHERE parent_id = 'cc-vendor'
  AND id NOT IN ('cc-vendor-new-york', 'cc-vendor-florida', 'cc-vendor-hamptons');
