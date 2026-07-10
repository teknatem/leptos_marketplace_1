ALTER TABLE document_raw_storage ADD COLUMN raw_hash TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_document_raw_storage_dedup
ON document_raw_storage(marketplace, document_type, document_no, raw_hash);

INSERT INTO sys_settings (key, value, created_at, updated_at)
VALUES ('raw_json_capture_enabled', 'false', datetime('now'), datetime('now'))
ON CONFLICT(key) DO NOTHING;
