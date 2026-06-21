CREATE TABLE IF NOT EXISTS sys_files_s3 (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    bucket TEXT NOT NULL,
    object_key TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    content_type TEXT,
    size_bytes INTEGER NOT NULL,
    etag TEXT,
    uploaded_by_user_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sys_files_s3_bucket_key_active
    ON sys_files_s3 (bucket, object_key)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_sys_files_s3_category
    ON sys_files_s3 (category)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_sys_files_s3_created_at
    ON sys_files_s3 (created_at);

CREATE INDEX IF NOT EXISTS idx_sys_files_s3_is_deleted
    ON sys_files_s3 (is_deleted);
