CREATE TABLE IF NOT EXISTS a026_wb_advert_daily (
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
    total_views INTEGER NOT NULL DEFAULT 0,
    total_clicks INTEGER NOT NULL DEFAULT 0,
    total_orders INTEGER NOT NULL DEFAULT 0,
    total_sum REAL NOT NULL DEFAULT 0,
    total_sum_price REAL NOT NULL DEFAULT 0,
    header_json TEXT NOT NULL DEFAULT '{}',
    totals_json TEXT NOT NULL DEFAULT '{}',
    unattributed_totals_json TEXT NOT NULL DEFAULT '{}',
    lines_json TEXT NOT NULL DEFAULT '[]',
    source_meta_json TEXT NOT NULL DEFAULT '{}',
    fetched_at TEXT NOT NULL DEFAULT '',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a026_wb_advert_daily_connection_date
    ON a026_wb_advert_daily(connection_id, document_date)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a026_wb_advert_daily_document_date
    ON a026_wb_advert_daily(document_date);

CREATE INDEX IF NOT EXISTS idx_a026_wb_advert_daily_connection
    ON a026_wb_advert_daily(connection_id);
