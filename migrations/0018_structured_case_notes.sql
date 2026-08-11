-- Structured Case Notes MVP.
--
-- Extends the existing free-text case_notes table into a lifecycle table for
-- draft/finalized/discarded/legacy notes, preserves existing rows as shared
-- legacy notes, adds immutable signed addenda, and keeps note lifecycle audit
-- metadata outside the generic Change Log.

-- ---------------------------------------------------------------------------
-- Case note lifecycle and structured fields
-- ---------------------------------------------------------------------------
ALTER TABLE case_notes
    ADD COLUMN state                  TEXT NOT NULL DEFAULT 'legacy',
    ADD COLUMN audience               TEXT NOT NULL DEFAULT 'shared_legacy',
    ADD COLUMN author_user_id          TEXT NOT NULL DEFAULT '',
    ADD COLUMN author_role_snapshot    TEXT NOT NULL DEFAULT '',
    ADD COLUMN updated_at              TEXT NOT NULL DEFAULT '',
    ADD COLUMN finalized_at            TEXT NOT NULL DEFAULT '',
    ADD COLUMN discarded_at            TEXT NOT NULL DEFAULT '',
    ADD COLUMN signature_name          TEXT NOT NULL DEFAULT '',
    ADD COLUMN accuracy_confirmed      BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN signature_signed_at     TEXT NOT NULL DEFAULT '',
    ADD COLUMN created_at_utc          TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN updated_at_utc          TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN finalized_at_utc        TIMESTAMPTZ,
    ADD COLUMN discarded_at_utc        TIMESTAMPTZ,
    ADD COLUMN signature_signed_at_utc TIMESTAMPTZ,
    ADD COLUMN activity_date           DATE,
    ADD COLUMN start_time              TIME,
    ADD COLUMN end_time                TIME,
    ADD COLUMN total_minutes           INTEGER,
    ADD COLUMN location                TEXT NOT NULL DEFAULT '',
    ADD COLUMN delayed_entry_reason    TEXT NOT NULL DEFAULT '',
    ADD COLUMN primary_interaction     TEXT NOT NULL DEFAULT '',
    ADD COLUMN contact_category        TEXT NOT NULL DEFAULT '',
    ADD COLUMN contact_direction       TEXT NOT NULL DEFAULT '',
    ADD COLUMN completion_outcome      TEXT NOT NULL DEFAULT '',
    ADD COLUMN participant_summary     TEXT NOT NULL DEFAULT '',
    ADD COLUMN service_areas           TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN purpose                 TEXT NOT NULL DEFAULT '',
    ADD COLUMN client_reported_info    TEXT NOT NULL DEFAULT '',
    ADD COLUMN verified_observed_info  TEXT NOT NULL DEFAULT '',
    ADD COLUMN information_sources     TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN actions_taken           TEXT NOT NULL DEFAULT '',
    ADD COLUMN outcome_response        TEXT NOT NULL DEFAULT '',
    ADD COLUMN progress_barriers       TEXT NOT NULL DEFAULT '',
    ADD COLUMN urgency                 TEXT NOT NULL DEFAULT '',
    ADD COLUMN urgency_details         TEXT NOT NULL DEFAULT '',
    ADD COLUMN next_steps              TEXT NOT NULL DEFAULT '',
    ADD COLUMN next_steps_not_applicable BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN narrative               TEXT NOT NULL DEFAULT '';

UPDATE case_notes
SET updated_at = created_at
WHERE updated_at = '';

-- Existing rows were historically shared free-text notes. New structured notes
-- are staff-only; the legacy audience stays explicit so old notes do not become
-- more or less visible by accident.
UPDATE case_notes
SET state = 'legacy', audience = 'shared_legacy'
WHERE state = 'legacy';

ALTER TABLE case_notes
    ADD CONSTRAINT case_notes_state_check
        CHECK (state IN ('draft', 'finalized', 'discarded', 'legacy')),
    ADD CONSTRAINT case_notes_audience_check
        CHECK (audience IN ('volunteer_only', 'shared_legacy')),
    ADD CONSTRAINT case_notes_state_audience_check
        CHECK ((state = 'legacy' AND audience = 'shared_legacy')
            OR (state <> 'legacy' AND audience = 'volunteer_only')),
    ADD CONSTRAINT case_notes_role_snapshot_check
        CHECK (author_role_snapshot = '' OR author_role_snapshot IN ('client', 'volunteer', 'operations_admin', 'site_admin')),
    ADD CONSTRAINT case_notes_primary_interaction_check
        CHECK (primary_interaction = '' OR primary_interaction IN (
            'telephone_call', 'email', 'text_message', 'videoconference',
            'in_person_meeting', 'court_appearance', 'client_accompaniment',
            'internal_casework', 'document_review', 'legal_research',
            'case_consultation', 'interdisciplinary_meeting', 'referral',
            'advocacy_outreach', 'outside_provider_communication',
            'attorney_communication', 'court_professional_communication',
            'government_agency_communication', 'attempted_contact', 'other'
        )),
    ADD CONSTRAINT case_notes_contact_category_check
        CHECK (contact_category = '' OR contact_category IN (
            'direct_client_contact', 'collateral_contact', 'internal_casework',
            'group_team_activity', 'attempted_contact', 'other', 'unknown',
            'client_declined', 'not_applicable'
        )),
    ADD CONSTRAINT case_notes_contact_direction_check
        CHECK (contact_direction = '' OR contact_direction IN (
            'incoming', 'outgoing', 'bidirectional', 'not_applicable', 'unknown'
        )),
    ADD CONSTRAINT case_notes_completion_outcome_check
        CHECK (completion_outcome = '' OR completion_outcome IN (
            'yes', 'no', 'partially', 'not_applicable', 'unknown'
        )),
    ADD CONSTRAINT case_notes_service_areas_check
        CHECK (service_areas <@ ARRAY[
            'legal', 'mental_health', 'domestic_violence_support', 'safety_planning',
            'social_services', 'housing', 'benefits_public_assistance',
            'financial_assistance', 'career_building', 'employment',
            'education_training', 'child_family_services', 'disability_services',
            'immigration', 'advocacy', 'referrals', 'case_management', 'other',
            'unknown', 'client_declined', 'not_applicable'
        ]::text[]),
    ADD CONSTRAINT case_notes_information_sources_check
        CHECK (information_sources <@ ARRAY[
            'client_reported', 'direct_observation', 'court_document',
            'attorney_communication', 'service_provider_communication',
            'government_agency_communication', 'police_child_protection_record',
            'other', 'unknown', 'client_declined', 'not_applicable'
        ]::text[]),
    ADD CONSTRAINT case_notes_urgency_check
        CHECK (urgency = '' OR urgency IN ('routine', 'elevated', 'urgent')),
    ADD CONSTRAINT case_notes_time_order_check
        CHECK (start_time IS NULL OR end_time IS NULL OR end_time > start_time),
    ADD CONSTRAINT case_notes_total_minutes_check
        CHECK (total_minutes IS NULL OR total_minutes > 0),
    ADD CONSTRAINT case_notes_next_steps_exclusive_check
        CHECK (NOT next_steps_not_applicable OR next_steps = ''),
    ADD CONSTRAINT case_notes_finalized_required_check
        CHECK (state <> 'finalized' OR (
            author_user_id <> ''
            AND author <> ''
            AND author_role_snapshot <> ''
            AND activity_date IS NOT NULL
            AND start_time IS NOT NULL
            AND end_time IS NOT NULL
            AND total_minutes IS NOT NULL
            AND primary_interaction <> ''
            AND contact_category <> ''
            AND cardinality(service_areas) > 0
            AND purpose <> ''
            AND actions_taken <> ''
            AND outcome_response <> ''
            AND urgency <> ''
            AND (urgency = 'routine' OR urgency_details <> '')
            AND (next_steps <> '' OR next_steps_not_applicable)
            AND narrative <> ''
            AND signature_name <> ''
            AND accuracy_confirmed
            AND signature_signed_at <> ''
            AND signature_signed_at_utc IS NOT NULL
            AND finalized_at <> ''
            AND finalized_at_utc IS NOT NULL
        )),
    ADD CONSTRAINT case_notes_discarded_tombstone_check
        CHECK (state <> 'discarded' OR (discarded_at <> '' AND discarded_at_utc IS NOT NULL));

ALTER TABLE case_notes
    ADD CONSTRAINT case_notes_id_case_id_unique UNIQUE (id, case_id);

CREATE INDEX case_notes_case_state_activity_idx
    ON case_notes(case_id, state, activity_date DESC NULLS LAST, seq DESC);
CREATE INDEX case_notes_case_author_state_idx ON case_notes(case_id, author_user_id, state);
CREATE INDEX case_notes_case_interaction_idx ON case_notes(case_id, primary_interaction);
CREATE INDEX case_notes_case_urgency_idx ON case_notes(case_id, urgency);

CREATE OR REPLACE FUNCTION prevent_terminal_case_note_update()
RETURNS trigger AS $$
BEGIN
    IF OLD.state IN ('finalized', 'legacy', 'discarded') THEN
        RAISE EXCEPTION 'case note % is immutable in state %', OLD.id, OLD.state;
    END IF;
    IF OLD.state = 'draft' AND NEW.state NOT IN ('draft', 'finalized', 'discarded') THEN
        RAISE EXCEPTION 'invalid case note state transition from % to %', OLD.state, NEW.state;
    END IF;
    IF NEW.audience <> OLD.audience THEN
        RAISE EXCEPTION 'case note audience cannot change';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER case_notes_prevent_terminal_update
BEFORE UPDATE ON case_notes
FOR EACH ROW EXECUTE FUNCTION prevent_terminal_case_note_update();

CREATE OR REPLACE FUNCTION prevent_case_note_delete()
RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'case notes are not deleted through the application';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER case_notes_prevent_delete
BEFORE DELETE ON case_notes
FOR EACH ROW EXECUTE FUNCTION prevent_case_note_delete();

-- ---------------------------------------------------------------------------
-- Immutable signed addenda
-- ---------------------------------------------------------------------------
CREATE TABLE case_note_addenda (
    id                   TEXT PRIMARY KEY,
    note_id              TEXT NOT NULL,
    case_id              TEXT NOT NULL,
    audience             TEXT NOT NULL,
    author_user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    author               TEXT NOT NULL,
    author_role_snapshot TEXT NOT NULL,
    reason               TEXT NOT NULL,
    information          TEXT NOT NULL,
    affected_categories  TEXT[] NOT NULL DEFAULT '{}',
    follow_up            TEXT NOT NULL DEFAULT '',
    signature_name       TEXT NOT NULL,
    signed_at            TEXT NOT NULL,
    signed_at_utc        TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq                  BIGSERIAL,
    CONSTRAINT case_note_addenda_note_fkey
        FOREIGN KEY (note_id, case_id)
        REFERENCES case_notes(id, case_id)
        ON DELETE RESTRICT,
    CONSTRAINT case_note_addenda_audience_check
        CHECK (audience IN ('volunteer_only', 'shared_legacy')),
    CONSTRAINT case_note_addenda_role_snapshot_check
        CHECK (author_role_snapshot IN ('volunteer', 'operations_admin', 'site_admin')),
    CONSTRAINT case_note_addenda_required_check
        CHECK (author_user_id <> '' AND author <> '' AND reason <> '' AND information <> ''
            AND cardinality(affected_categories) > 0 AND signature_name <> '' AND signed_at <> ''),
    CONSTRAINT case_note_addenda_categories_check
        CHECK (affected_categories <@ ARRAY[
            'activity_timing', 'interaction', 'participants', 'service_areas',
            'reported_information', 'observed_information', 'actions', 'outcome',
            'urgency', 'next_steps', 'narrative', 'other'
        ]::text[])
);

CREATE OR REPLACE FUNCTION validate_case_note_addendum()
RETURNS trigger AS $$
DECLARE
    parent_state TEXT;
    parent_audience TEXT;
BEGIN
    SELECT state, audience INTO parent_state, parent_audience
    FROM case_notes
    WHERE id = NEW.note_id AND case_id = NEW.case_id
    FOR UPDATE;

    IF parent_state NOT IN ('finalized', 'legacy') THEN
        RAISE EXCEPTION 'addenda require a finalized or legacy note';
    END IF;
    IF NEW.audience <> parent_audience THEN
        RAISE EXCEPTION 'addendum audience must match its parent note';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER case_note_addenda_validate_parent
BEFORE INSERT ON case_note_addenda
FOR EACH ROW EXECUTE FUNCTION validate_case_note_addendum();

CREATE INDEX case_note_addenda_note_idx ON case_note_addenda(note_id, seq ASC);
CREATE INDEX case_note_addenda_case_idx ON case_note_addenda(case_id, seq DESC);

CREATE OR REPLACE FUNCTION prevent_case_note_addendum_change()
RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'case note addenda are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER case_note_addenda_prevent_update
BEFORE UPDATE ON case_note_addenda
FOR EACH ROW EXECUTE FUNCTION prevent_case_note_addendum_change();

CREATE TRIGGER case_note_addenda_prevent_delete
BEFORE DELETE ON case_note_addenda
FOR EACH ROW EXECUTE FUNCTION prevent_case_note_addendum_change();

-- ---------------------------------------------------------------------------
-- Restricted, content-free note audit
-- ---------------------------------------------------------------------------
CREATE TABLE case_note_audit_log (
    id                   TEXT PRIMARY KEY,
    note_id              TEXT NOT NULL,
    case_id              TEXT NOT NULL,
    addendum_id          TEXT NOT NULL DEFAULT '',
    actor_user_id        TEXT NOT NULL,
    actor                TEXT NOT NULL,
    actor_role_snapshot  TEXT NOT NULL,
    action               TEXT NOT NULL,
    at                   TEXT NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    metadata             JSONB NOT NULL DEFAULT '{}'::jsonb,
    seq                  BIGSERIAL,
    CONSTRAINT case_note_audit_role_snapshot_check
        CHECK (actor_role_snapshot IN ('volunteer', 'operations_admin', 'site_admin')),
    CONSTRAINT case_note_audit_action_check
        CHECK (action IN (
            'create_draft', 'save_draft', 'discard_draft', 'finalize_note',
            'admin_inspect_draft', 'add_addendum', 'export_note'
        )),
    CONSTRAINT case_note_audit_metadata_object_check
        CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX case_note_audit_case_idx ON case_note_audit_log(case_id, seq DESC);
CREATE INDEX case_note_audit_note_idx ON case_note_audit_log(note_id, seq DESC);
CREATE INDEX case_note_audit_action_idx ON case_note_audit_log(action, created_at DESC);
