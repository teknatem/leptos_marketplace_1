CREATE TABLE IF NOT EXISTS a028_missing_cost_registry (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL,
    description TEXT NOT NULL,
    comment TEXT,
    document_no TEXT NOT NULL,
    document_date TEXT NOT NULL,
    lines_json TEXT,
    is_deleted BOOLEAN NOT NULL DEFAULT 0,
    is_posted BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a028_document_date
    ON a028_missing_cost_registry(document_date);

CREATE INDEX IF NOT EXISTS idx_a028_document_no
    ON a028_missing_cost_registry(document_no);

CREATE INDEX IF NOT EXISTS idx_a028_is_deleted_date
    ON a028_missing_cost_registry(is_deleted, document_date DESC);
