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

-- Initial taxonomy from the client specification. Both parent and child rows
-- are assignable, allowing broad and precise multi-category searches.
INSERT INTO contact_categories (id, name) VALUES
    ('cc-legal', 'Legal'),
    ('cc-social-services', 'Social Services'),
    ('cc-education', 'Education'),
    ('cc-medical-mental-health', 'Medical / Mental Health'),
    ('cc-government-public-affairs', 'Government / Public Affairs'),
    ('cc-media-entertainment', 'Media / Entertainment'),
    ('cc-philanthropy-fundraising', 'Philanthropy / Fundraising'),
    ('cc-business-corporate', 'Business / Corporate'),
    ('cc-nonprofit-community', 'Nonprofit / Community');

INSERT INTO contact_categories (id, parent_id, name) VALUES
    ('cc-law-firms', 'cc-legal', 'Law firms'),
    ('cc-individual-attorneys', 'cc-legal', 'Individual attorneys'),
    ('cc-family-law', 'cc-legal', 'Family law'),
    ('cc-domestic-violence-legal', 'cc-legal', 'Domestic violence'),
    ('cc-matrimonial-law', 'cc-legal', 'Matrimonial law'),
    ('cc-appellate-law', 'cc-legal', 'Appellate law'),
    ('cc-law-schools', 'cc-legal', 'Law schools'),
    ('cc-law-school-clinics', 'cc-legal', 'Law school clinics'),
    ('cc-legal-services-organizations', 'cc-legal', 'Legal services organizations'),
    ('cc-housing', 'cc-social-services', 'Housing'),
    ('cc-public-benefits', 'cc-social-services', 'Public benefits'),
    ('cc-domestic-violence-services', 'cc-social-services', 'Domestic violence services'),
    ('cc-mental-health-services', 'cc-social-services', 'Mental health'),
    ('cc-child-family-services', 'cc-social-services', 'Child / family services'),
    ('cc-food-assistance', 'cc-social-services', 'Food assistance'),
    ('cc-financial-assistance', 'cc-social-services', 'Financial assistance'),
    ('cc-employment-career-services', 'cc-social-services', 'Employment / career services'),
    ('cc-community-based-organizations', 'cc-social-services', 'Community-based organizations'),
    ('cc-colleges-universities', 'cc-education', 'Colleges / universities'),
    ('cc-graduate-schools', 'cc-education', 'Graduate schools'),
    ('cc-social-work-schools', 'cc-education', 'Social work schools'),
    ('cc-education-law-schools', 'cc-education', 'Law schools'),
    ('cc-field-placement-offices', 'cc-education', 'Internship / field placement offices'),
    ('cc-career-services', 'cc-education', 'Career services'),
    ('cc-alumni-associations', 'cc-education', 'Alumni associations'),
    ('cc-faculty-professors', 'cc-education', 'Faculty / professors'),
    ('cc-therapists', 'cc-medical-mental-health', 'Therapists'),
    ('cc-social-workers', 'cc-medical-mental-health', 'Social workers'),
    ('cc-psychologists', 'cc-medical-mental-health', 'Psychologists'),
    ('cc-psychiatrists', 'cc-medical-mental-health', 'Psychiatrists'),
    ('cc-physicians', 'cc-medical-mental-health', 'Physicians'),
    ('cc-clinics-hospitals', 'cc-medical-mental-health', 'Clinics / hospitals'),
    ('cc-public-officials', 'cc-government-public-affairs', 'Public officials'),
    ('cc-retired-public-officials', 'cc-government-public-affairs', 'Retired public officials'),
    ('cc-government-agencies', 'cc-government-public-affairs', 'Government agencies'),
    ('cc-legislative-offices', 'cc-government-public-affairs', 'Legislative offices'),
    ('cc-policy-advocacy-organizations', 'cc-government-public-affairs', 'Policy / advocacy organizations'),
    ('cc-actors', 'cc-media-entertainment', 'Actors'),
    ('cc-musicians', 'cc-media-entertainment', 'Musicians'),
    ('cc-tv-hosts', 'cc-media-entertainment', 'TV hosts'),
    ('cc-newscasters', 'cc-media-entertainment', 'Newscasters'),
    ('cc-reporters-journalists', 'cc-media-entertainment', 'Reporters / journalists'),
    ('cc-producers', 'cc-media-entertainment', 'Producers'),
    ('cc-media-organizations', 'cc-media-entertainment', 'Media organizations'),
    ('cc-philanthropists', 'cc-philanthropy-fundraising', 'Philanthropists'),
    ('cc-foundations', 'cc-philanthropy-fundraising', 'Foundations'),
    ('cc-corporate-sponsors', 'cc-philanthropy-fundraising', 'Corporate sponsors'),
    ('cc-donors', 'cc-philanthropy-fundraising', 'Donors'),
    ('cc-prospective-donors', 'cc-philanthropy-fundraising', 'Prospective donors'),
    ('cc-funders-grantmakers', 'cc-philanthropy-fundraising', 'Funders / grantmaking organizations'),
    ('cc-companies', 'cc-business-corporate', 'Companies'),
    ('cc-corporate-partners', 'cc-business-corporate', 'Corporate partners'),
    ('cc-professional-services', 'cc-business-corporate', 'Professional services'),
    ('cc-vendors', 'cc-business-corporate', 'Vendors'),
    ('cc-prospective-sponsors', 'cc-business-corporate', 'Prospective sponsors'),
    ('cc-nonprofit-organizations', 'cc-nonprofit-community', 'Nonprofit organizations'),
    ('cc-advocacy-organizations', 'cc-nonprofit-community', 'Advocacy organizations'),
    ('cc-community-organizations', 'cc-nonprofit-community', 'Community organizations'),
    ('cc-referral-partners', 'cc-nonprofit-community', 'Referral partners'),
    ('cc-prospective-partners', 'cc-nonprofit-community', 'Prospective partners');
