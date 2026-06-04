CREATE TABLE IF NOT EXISTS a034_ym_realization (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,
    document_no TEXT NOT NULL DEFAULT '',
    document_date TEXT NOT NULL DEFAULT '',
    connection_id TEXT NOT NULL DEFAULT '',
    organization_id TEXT NOT NULL DEFAULT '',
    marketplace_id TEXT NOT NULL DEFAULT '',
    lines_count INTEGER NOT NULL DEFAULT 0,
    total_sales_revenue REAL NOT NULL DEFAULT 0,
    total_return_revenue REAL NOT NULL DEFAULT 0,
    net_revenue REAL NOT NULL DEFAULT 0,
    header_json TEXT NOT NULL DEFAULT '{}',
    totals_json TEXT NOT NULL DEFAULT '{}',
    lines_json TEXT NOT NULL DEFAULT '[]',
    source_meta_json TEXT NOT NULL DEFAULT '{}',
    fetched_at TEXT NOT NULL DEFAULT '',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a034_ym_realization_connection_date
    ON a034_ym_realization(connection_id, document_date)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a034_ym_realization_document_date
    ON a034_ym_realization(document_date);

CREATE INDEX IF NOT EXISTS idx_a034_ym_realization_connection
    ON a034_ym_realization(connection_id);
