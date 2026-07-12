-- Add file-backed evidence: previously the `evidence` table held only metadata
-- (name, uploader, description). These columns capture a real uploaded file that
-- lives in Azure Blob Storage — the DB keeps the metadata + a pointer, never the
-- bytes.
--
-- All columns are added with defaults so existing metadata-only evidence rows
-- stay valid (they simply have no backing blob: empty `blob_path`).
--
--   original_filename  the user-supplied file name (display + download name)
--   content_type       the validated MIME type (e.g. application/pdf)
--   size_bytes         file size, for display and quota/limit checks
--   sha256             hex SHA-256 of the bytes, for integrity + future dedup
--   blob_path          blob name within the evidence container (empty = no file)
--   status             lifecycle: 'stored' (default), plus forward-compatible
--                      'pending' / 'quarantined' / 'infected' hooks so malware
--                      scanning (e.g. Microsoft Defender for Storage) can be
--                      layered on later without another migration.

ALTER TABLE evidence
    ADD COLUMN original_filename TEXT   NOT NULL DEFAULT '',
    ADD COLUMN content_type      TEXT   NOT NULL DEFAULT '',
    ADD COLUMN size_bytes        BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN sha256            TEXT   NOT NULL DEFAULT '',
    ADD COLUMN blob_path         TEXT   NOT NULL DEFAULT '',
    ADD COLUMN status            TEXT   NOT NULL DEFAULT 'stored';
