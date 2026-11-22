CREATE TABLE case_record_files (
    kind          TEXT NOT NULL,
    record_id     TEXT NOT NULL,
    version       INTEGER NOT NULL,
    case_id       TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL,
    file_name     TEXT NOT NULL,
    web_url       TEXT NOT NULL DEFAULT '',
    filed_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (kind, record_id, version),
    CONSTRAINT case_record_files_kind_check CHECK (kind IN ('legal_memo', 'closing_note'))
);

CREATE INDEX case_record_files_case_path_idx ON case_record_files (case_id, relative_path);
