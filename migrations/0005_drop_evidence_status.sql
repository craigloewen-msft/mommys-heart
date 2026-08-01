-- The evidence.status column is dropped. It only ever held 'stored' / 'awaiting',
-- both fully determined by whether blob_path is set, so it carried no independent
-- information; `has_file` is now derived from blob_path directly. The forward-
-- compatible malware-scan states ('pending' / 'quarantined' / 'infected') were
-- never used, and can reintroduce a column if scanning is ever added.
ALTER TABLE evidence DROP COLUMN status;
