-- Rule property fields can demand an answer.
--
-- A field is required by default: adding one to a rule is a statement that
-- contacts of that kind should carry it. Unticking makes it a prompt instead.
--
-- Enforced when a contact is created, where the form collects the values up
-- front. Contacts that already exist are not blocked by a field added after
-- the fact -- they show it as required and unanswered instead, so classifying
-- someone never becomes impossible.
ALTER TABLE contact_rule_fields
    ADD COLUMN required BOOLEAN NOT NULL DEFAULT true;
