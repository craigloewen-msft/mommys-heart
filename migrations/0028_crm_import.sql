-- Spreadsheet imports: the staged upload, the task, and its per-row outcomes.

-- A parsed sheet held between "upload" and "import", so the mapping can be
-- retargeted and re-planned without asking for the file again. Short-lived:
-- purged on a timer, and always re-readable from the original file if lost.
CREATE TABLE crm_import_uploads (
    id             TEXT PRIMARY KEY,
    subject        TEXT NOT NULL,
    file_name      TEXT NOT NULL,
    sheet_name     TEXT NOT NULL DEFAULT '',
    headers        JSONB NOT NULL,
    rows           JSONB NOT NULL,
    row_count      INTEGER NOT NULL,
    uploaded_by_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT crm_import_uploads_subject_check CHECK (subject IN ('contact', 'organization')),
    CONSTRAINT crm_import_uploads_row_count_check CHECK (row_count > 0)
);

CREATE INDEX crm_import_uploads_created_at_idx ON crm_import_uploads (created_at);

CREATE TABLE crm_import_tasks (
    id                       TEXT PRIMARY KEY,
    status                   TEXT NOT NULL DEFAULT 'queued',
    subject                  TEXT NOT NULL,
    file_name                TEXT NOT NULL,
    -- The confirmed mapping and policy, kept verbatim so a finished import can
    -- always be explained after the fact.
    mapping                  JSONB NOT NULL,
    policy                   JSONB NOT NULL,
    created_by_id            TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_by_name          TEXT NOT NULL,
    row_total                INTEGER NOT NULL,
    created_count            INTEGER NOT NULL DEFAULT 0,
    updated_count            INTEGER NOT NULL DEFAULT 0,
    skipped_count            INTEGER NOT NULL DEFAULT 0,
    failed_count             INTEGER NOT NULL DEFAULT 0,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at               TIMESTAMPTZ,
    completed_at             TIMESTAMPTZ,
    cancel_requested_at      TIMESTAMPTZ,
    cancel_requested_by_id   TEXT REFERENCES users(id) ON DELETE SET NULL,
    cancel_requested_by_name TEXT NOT NULL DEFAULT '',
    error                    TEXT NOT NULL DEFAULT '',
    seq                      BIGSERIAL,
    CONSTRAINT crm_import_tasks_status_check CHECK (status IN (
        'queued', 'running', 'cancelling', 'completed', 'cancelled', 'failed'
    )),
    CONSTRAINT crm_import_tasks_subject_check CHECK (subject IN ('contact', 'organization')),
    CONSTRAINT crm_import_tasks_counts_check CHECK (
        row_total > 0
        AND created_count >= 0
        AND updated_count >= 0
        AND skipped_count >= 0
        AND failed_count >= 0
        AND created_count + updated_count + skipped_count + failed_count <= row_total
    )
);

CREATE INDEX crm_import_tasks_seq_idx ON crm_import_tasks (seq DESC);

-- One row per source row. This is what makes "row 412: invalid email"
-- reportable instead of a bare failure count.
CREATE TABLE crm_import_rows (
    task_id   TEXT NOT NULL REFERENCES crm_import_tasks(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    status    TEXT NOT NULL DEFAULT 'pending',
    record_id TEXT NOT NULL DEFAULT '',
    label     TEXT NOT NULL DEFAULT '',
    error     TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (task_id, position),
    CONSTRAINT crm_import_rows_position_check CHECK (position > 0),
    CONSTRAINT crm_import_rows_status_check CHECK (status IN (
        'pending', 'created', 'updated', 'skipped', 'failed'
    ))
);

CREATE INDEX crm_import_rows_failed_idx
    ON crm_import_rows (task_id, position)
    WHERE status = 'failed';
