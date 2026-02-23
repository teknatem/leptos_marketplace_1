-- Migration: fix p908_wb_goods_prices price columns from INTEGER to REAL
-- WB API returns prices as float (e.g. 28071.04), not integer

DROP TABLE IF EXISTS p908_wb_goods_prices;

CREATE TABLE IF NOT EXISTS p908_wb_goods_prices (
    nm_id               INTEGER PRIMARY KEY,
    connection_mp_ref   TEXT    NOT NULL,
    vendor_code         TEXT,
    discount            INTEGER,
    editable_size_price INTEGER NOT NULL DEFAULT 0,
    price               REAL,               -- price of first size (rubles, float)
    discounted_price    REAL,               -- discounted price of first size
    sizes_json          TEXT    NOT NULL DEFAULT '[]',
    fetched_at          TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_p908_connection  ON p908_wb_goods_prices (connection_mp_ref);
CREATE INDEX IF NOT EXISTS idx_p908_vendor_code ON p908_wb_goods_prices (vendor_code);
