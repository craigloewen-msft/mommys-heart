-- Saved reports: a report the user chose to keep, so it can be re-run without
-- asking the AI again. The final SELECT and its chart spec are what make the
-- re-run possible; the prose request is kept for context only.
CREATE TABLE saved_reports (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL CHECK (btrim(title) <> ''),
    request     TEXT NOT NULL DEFAULT '',
    summary     TEXT NOT NULL DEFAULT '',
    sql_text    TEXT NOT NULL,
    chart       JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by  TEXT NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_run_at TIMESTAMPTZ
);

CREATE INDEX saved_reports_created_idx ON saved_reports (created_at DESC);
