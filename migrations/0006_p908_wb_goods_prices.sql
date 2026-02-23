-- Migration: p908_wb_goods_prices
-- Stores WB goods prices fetched from GET /api/v2/list/goods/filter
-- One row per nm_id (upserted on import), sizes stored as JSON

CREATE TABLE IF NOT EXISTS p908_wb_goods_prices (
    nm_id               INTEGER PRIMARY KEY,
    connection_mp_ref   TEXT    NOT NULL,
    vendor_code         TEXT,
    discount            INTEGER,
    editable_size_price INTEGER NOT NULL DEFAULT 0,
    price               INTEGER,            -- price of first size (kopecks/rubles per WB)
    discounted_price    INTEGER,            -- discounted price of first size
    sizes_json          TEXT    NOT NULL DEFAULT '[]',
    fetched_at          TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_p908_connection ON p908_wb_goods_prices (connection_mp_ref);
CREATE INDEX IF NOT EXISTS idx_p908_vendor_code ON p908_wb_goods_prices (vendor_code);
