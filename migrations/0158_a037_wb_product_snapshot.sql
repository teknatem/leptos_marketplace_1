CREATE TABLE IF NOT EXISTS a037_wb_product_snapshot (
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
    total_stock_wb INTEGER NOT NULL DEFAULT 0,
    total_stock_mp INTEGER NOT NULL DEFAULT 0,
    total_balance_sum REAL NOT NULL DEFAULT 0,
    header_json TEXT NOT NULL DEFAULT '{}',
    totals_json TEXT NOT NULL DEFAULT '{}',
    lines_json TEXT NOT NULL DEFAULT '[]',
    source_meta_json TEXT NOT NULL DEFAULT '{}',
    fetched_at TEXT NOT NULL DEFAULT '',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a037_wb_product_snapshot_connection_date
    ON a037_wb_product_snapshot(connection_id, document_date)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a037_wb_product_snapshot_document_date
    ON a037_wb_product_snapshot(document_date);

CREATE INDEX IF NOT EXISTS idx_a037_wb_product_snapshot_connection
    ON a037_wb_product_snapshot(connection_id);
