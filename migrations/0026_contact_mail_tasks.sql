-- Durable, site-wide contact mail campaigns and their immutable recipient snapshots.
CREATE TABLE contact_mail_tasks (
    id                       TEXT PRIMARY KEY,
    status                   TEXT NOT NULL DEFAULT 'queued',
    subject                  TEXT NOT NULL,
    body                     TEXT NOT NULL,
    created_by_id            TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_by_name          TEXT NOT NULL,
    recipient_total          INTEGER NOT NULL,
    accepted_count           INTEGER NOT NULL DEFAULT 0,
    failed_count             INTEGER NOT NULL DEFAULT 0,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at               TIMESTAMPTZ,
    completed_at             TIMESTAMPTZ,
    next_send_at             TIMESTAMPTZ,
    cancel_requested_at      TIMESTAMPTZ,
    cancel_requested_by_id   TEXT REFERENCES users(id) ON DELETE SET NULL,
    cancel_requested_by_name TEXT NOT NULL DEFAULT '',
    error                    TEXT NOT NULL DEFAULT '',
    seq                      BIGSERIAL,
    CONSTRAINT contact_mail_tasks_status_check CHECK (status IN (
        'queued', 'running', 'cancelling', 'completed', 'cancelled', 'failed'
    )),
    CONSTRAINT contact_mail_tasks_subject_check CHECK (btrim(subject) <> ''),
    CONSTRAINT contact_mail_tasks_body_check CHECK (btrim(body) <> ''),
    CONSTRAINT contact_mail_tasks_counts_check CHECK (
        recipient_total > 0
        AND accepted_count >= 0
        AND failed_count >= 0
        AND accepted_count + failed_count <= recipient_total
    )
);

CREATE INDEX contact_mail_tasks_seq_idx ON contact_mail_tasks (seq DESC);

CREATE TABLE contact_mail_recipients (
    task_id            TEXT NOT NULL REFERENCES contact_mail_tasks(id) ON DELETE CASCADE,
    position           INTEGER NOT NULL,
    contact_id         TEXT REFERENCES contacts(id) ON DELETE SET NULL,
    name               TEXT NOT NULL,
    email              TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'pending',
    attempt_started_at TIMESTAMPTZ,
    finished_at        TIMESTAMPTZ,
    error              TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (task_id, position),
    CONSTRAINT contact_mail_recipients_position_check CHECK (position > 0),
    CONSTRAINT contact_mail_recipients_email_check CHECK (btrim(email) <> ''),
    CONSTRAINT contact_mail_recipients_status_check CHECK (status IN (
        'pending', 'sending', 'accepted', 'failed'
    ))
);

CREATE UNIQUE INDEX contact_mail_recipients_unique_email_idx
    ON contact_mail_recipients (task_id, lower(btrim(email)));
CREATE INDEX contact_mail_recipients_pending_idx
    ON contact_mail_recipients (task_id, position)
    WHERE status = 'pending';
