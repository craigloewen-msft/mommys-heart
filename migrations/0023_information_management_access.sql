-- One stored grant for the information areas in the navbar. Existing staff keep
-- today's access; clients stay denied; new accounts default denied.
ALTER TABLE users
    ADD COLUMN information_management_access BOOLEAN NOT NULL DEFAULT false;

UPDATE users
SET information_management_access = role IN ('volunteer', 'operations_admin', 'site_admin');
