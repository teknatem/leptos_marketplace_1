-- Migration: a020_wb_promotion - WB Calendar Promotions aggregate

CREATE TABLE IF NOT EXISTS a020_wb_promotion (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,

    -- Document header
    document_no TEXT NOT NULL DEFAULT '',
    connection_id TEXT NOT NULL DEFAULT '',
    organization_id TEXT NOT NULL DEFAULT '',

    -- Denormalized promotion data for fast queries
    promotion_id INTEGER NOT NULL DEFAULT 0,
    name TEXT NOT NULL DEFAULT '',
    promotion_description TEXT,
    start_date_time TEXT NOT NULL DEFAULT '',
    end_date_time TEXT NOT NULL DEFAULT '',
    promotion_type TEXT,
    exception_products_count INTEGER,
    in_promo_action_total INTEGER,

    -- JSON storage for full structured data
    header_json TEXT NOT NULL DEFAULT '{}',
    data_json TEXT NOT NULL DEFAULT '{}',
    nomenclatures_json TEXT NOT NULL DEFAULT '[]',
    source_meta_json TEXT NOT NULL DEFAULT '{}',

    -- Raw storage reference
    raw_payload_ref TEXT NOT NULL DEFAULT '',
    fetched_at TEXT NOT NULL DEFAULT '',

    -- Base fields
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

-- Dedup index: one promotion per connection
CREATE UNIQUE INDEX IF NOT EXISTS idx_a020_wb_promotion_dedup
    ON a020_wb_promotion(connection_id, promotion_id)
    WHERE is_deleted = 0;

-- Index for date range filtering
CREATE INDEX IF NOT EXISTS idx_a020_wb_promotion_dates
    ON a020_wb_promotion(start_date_time, end_date_time);

-- Index for connection filtering
CREATE INDEX IF NOT EXISTS idx_a020_wb_promotion_connection
    ON a020_wb_promotion(connection_id);
