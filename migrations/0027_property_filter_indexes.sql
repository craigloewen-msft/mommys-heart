-- Indexes for filtering contacts and organizations by their custom properties.
--
-- Matching normalizes both sides with lower(btrim(...)) so "Location" and
-- "location" are one facet, which means a plain column index cannot serve the
-- lookup; these are expression indexes over the same expressions the queries use.
--
-- The keyword-into-values search stays a sequential scan: it is a leading-%
-- ILIKE, which no b-tree can serve and which would need the pg_trgm extension
-- (see 0019). At this data size that matches what the contacts keyword search
-- already does across its own columns.

CREATE INDEX contact_properties_key_value_idx
    ON contact_properties (lower(btrim(key)), lower(btrim(value)));
CREATE INDEX contact_properties_record_key_idx
    ON contact_properties (contact_id, lower(btrim(key)));

CREATE INDEX organization_properties_key_value_idx
    ON organization_properties (lower(btrim(key)), lower(btrim(value)));
CREATE INDEX organization_properties_record_key_idx
    ON organization_properties (organization_id, lower(btrim(key)));
