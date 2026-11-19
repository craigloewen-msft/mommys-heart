CREATE TABLE legal_memos (
    id               TEXT PRIMARY KEY,
    case_id          TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    title            TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'draft',
    version          INTEGER NOT NULL DEFAULT 1,
    attorney_user_id TEXT REFERENCES users(id) ON DELETE RESTRICT,
    attorney_name    TEXT NOT NULL DEFAULT '',
    author_id        TEXT REFERENCES users(id) ON DELETE RESTRICT,
    author_name      TEXT NOT NULL,
    content          JSONB NOT NULL DEFAULT '{}'::jsonb,
    review           JSONB NOT NULL DEFAULT '{}'::jsonb,
    approved_by_name TEXT NOT NULL DEFAULT '',
    approved_at      TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq              BIGSERIAL,
    CONSTRAINT legal_memos_title_check CHECK (btrim(title) <> ''),
    CONSTRAINT legal_memos_status_check CHECK (status IN (
        'draft', 'in_review', 'revisions_requested', 'approved'
    )),
    CONSTRAINT legal_memos_version_check CHECK (version >= 1),
    CONSTRAINT legal_memos_content_check CHECK (jsonb_typeof(content) = 'object'),
    CONSTRAINT legal_memos_review_check CHECK (jsonb_typeof(review) = 'object')
);

CREATE INDEX legal_memos_case_id_idx ON legal_memos (case_id, seq);

CREATE FUNCTION prevent_approved_legal_memo_update() RETURNS trigger AS $$
BEGIN
    IF OLD.status = 'approved'
       AND NOT (NEW.status = 'draft' AND NEW.version = OLD.version + 1) THEN
        RAISE EXCEPTION 'An approved legal memo cannot be changed.';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER legal_memos_approved_immutable
    BEFORE UPDATE ON legal_memos
    FOR EACH ROW EXECUTE FUNCTION prevent_approved_legal_memo_update();

CREATE TABLE legal_memo_versions (
    id          TEXT PRIMARY KEY,
    memo_id     TEXT NOT NULL REFERENCES legal_memos(id) ON DELETE CASCADE,
    version     INTEGER NOT NULL,
    event       TEXT NOT NULL,
    title       TEXT NOT NULL,
    content     JSONB NOT NULL,
    attorney_name TEXT NOT NULL DEFAULT '',
    actor_id    TEXT REFERENCES users(id) ON DELETE RESTRICT,
    actor_name  TEXT NOT NULL,
    comment     TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    seq         BIGSERIAL,
    CONSTRAINT legal_memo_versions_event_check CHECK (event IN (
        'sent', 'revisions_requested', 'approved'
    )),
    CONSTRAINT legal_memo_versions_content_check CHECK (jsonb_typeof(content) = 'object')
);

CREATE INDEX legal_memo_versions_memo_id_idx ON legal_memo_versions (memo_id, seq);
CREATE UNIQUE INDEX legal_memo_versions_one_approval_idx
    ON legal_memo_versions (memo_id, version) WHERE event = 'approved';

CREATE FUNCTION prevent_legal_memo_version_change() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'Legal memo versions cannot be changed or deleted.';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER legal_memo_versions_no_update
    BEFORE UPDATE ON legal_memo_versions
    FOR EACH ROW EXECUTE FUNCTION prevent_legal_memo_version_change();

CREATE TRIGGER legal_memo_versions_no_delete
    BEFORE DELETE ON legal_memo_versions
    FOR EACH ROW EXECUTE FUNCTION prevent_legal_memo_version_change();
