ALTER TABLE legal_memos
    ADD COLUMN review JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD CONSTRAINT legal_memos_review_check CHECK (jsonb_typeof(review) = 'object');

ALTER TABLE legal_memo_versions
    ADD COLUMN review JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD CONSTRAINT legal_memo_versions_review_check CHECK (jsonb_typeof(review) = 'object');

ALTER TABLE legal_memo_versions
    DROP CONSTRAINT legal_memo_versions_event_check,
    ADD CONSTRAINT legal_memo_versions_event_check CHECK (event IN (
        'sent', 'revisions_requested', 'approved', 'withdrawn', 'started'
    ));

CREATE OR REPLACE FUNCTION prevent_approved_legal_memo_update() RETURNS trigger AS $$
BEGIN
    IF OLD.status = 'approved'
       AND NOT (NEW.status = 'draft' AND NEW.version = OLD.version + 1
                AND NEW.case_id = OLD.case_id AND NEW.title = OLD.title
                AND NEW.content = OLD.content
                AND NEW.attorney_user_id IS NOT DISTINCT FROM OLD.attorney_user_id
                AND NEW.author_id = OLD.author_id)
       AND (NEW.status, NEW.version, NEW.case_id, NEW.title, NEW.content, NEW.author_id,
            NEW.attorney_user_id, NEW.attorney_name, NEW.approved_by_name, NEW.approved_at,
            NEW.review)
           IS DISTINCT FROM
           (OLD.status, OLD.version, OLD.case_id, OLD.title, OLD.content, OLD.author_id,
            OLD.attorney_user_id, OLD.attorney_name, OLD.approved_by_name, OLD.approved_at,
            OLD.review) THEN
        RAISE EXCEPTION 'An approved legal memo cannot be changed.';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE case_record_files (
    kind          TEXT NOT NULL,
    record_id     TEXT NOT NULL,
    version       INTEGER NOT NULL,
    case_id       TEXT NOT NULL REFERENCES cases(id) ON DELETE RESTRICT,
    relative_path TEXT NOT NULL,
    file_name     TEXT NOT NULL,
    filed_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (kind, record_id, version),
    CONSTRAINT case_record_files_kind_check CHECK (kind IN ('legal_memo'))
);

CREATE INDEX case_record_files_case_path_idx ON case_record_files (case_id, relative_path);
