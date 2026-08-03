-- Self-reported volunteer service time. Durations are stored as whole minutes
-- so totals remain exact while the UI can present them as hours.
CREATE TABLE volunteer_hours (
    id               TEXT PRIMARY KEY,
    user_id          TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    service_date     DATE NOT NULL,
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    description      TEXT NOT NULL DEFAULT '',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq              BIGSERIAL
);

CREATE INDEX volunteer_hours_user_date_idx
    ON volunteer_hours(user_id, service_date DESC, seq DESC);