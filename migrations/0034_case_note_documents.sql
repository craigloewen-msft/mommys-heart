-- Every finalized case note is also filed as a document in the case's
-- "Case Notes" folder in the SharePoint library.
--
-- The library holds the *document*; this table holds only the pointer to it.
-- That is the same doctrine migration 0033 applies to a case's folder: the
-- library is the source of truth for the file, and a mirror of its listing or
-- its bytes here would only be a second copy to drift.
--
-- It is a separate table rather than columns on `case_notes` because a
-- finalized note row is immutable by trigger
-- (`case_notes_prevent_terminal_update`): filing happens after the note commits,
-- and re-filing happens again whenever an addendum is added, so the pointer has
-- to live somewhere that is allowed to change.
CREATE TABLE case_note_documents (
    -- One filed document per note. Re-filing updates this row in place.
    note_id          TEXT PRIMARY KEY REFERENCES case_notes(id) ON DELETE CASCADE,
    case_id          TEXT NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    -- File name inside the case's "Case Notes" folder. Stable across re-filings,
    -- so an addendum replaces the document rather than adding a second one.
    file_name        TEXT NOT NULL,
    -- Browser link to the document in SharePoint. Empty for the on-disk store.
    web_url          TEXT NOT NULL DEFAULT '',
    -- How many addenda the filed document already includes. This is what makes
    -- "stale" answerable without downloading and parsing the file: a note with
    -- more addenda than this has a document that needs writing again.
    addenda_included INTEGER NOT NULL DEFAULT 0,
    filed_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX case_note_documents_case_idx ON case_note_documents(case_id);
