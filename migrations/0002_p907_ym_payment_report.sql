-- 0002 p907_ym_payment_report
-- YM (Yandex Market) Payment Report projection table
-- Source: POST v2/reports/united-netting/generate + GET v2/reports/info/{reportId} + CSV download

CREATE TABLE IF NOT EXISTS p907_ym_payment_report (
    -- Primary key: transaction_id from YM report
    transaction_id TEXT NOT NULL PRIMARY KEY,

    -- Metadata
    connection_mp_ref TEXT NOT NULL DEFAULT '',
    organization_ref TEXT NOT NULL DEFAULT '',

    -- Business info
    business_id INTEGER,
    partner_id INTEGER,
    shop_name TEXT,
    inn TEXT,
    model TEXT,

    -- Transaction info
    transaction_date TEXT,
    transaction_type TEXT,
    transaction_source TEXT,
    transaction_sum REAL,
    payment_status TEXT,

    -- Order info
    order_id INTEGER,
    shop_order_id TEXT,
    order_creation_date TEXT,
    order_delivery_date TEXT,
    order_type TEXT,

    -- Product/service info
    shop_sku TEXT,
    offer_or_service_name TEXT,
    count INTEGER,

    -- Bank / Act info
    act_id INTEGER,
    act_date TEXT,
    bank_order_id INTEGER,
    bank_order_date TEXT,
    bank_sum REAL,

    -- Extra
    claim_number TEXT,
    bonus_account_year_month TEXT,
    comments TEXT,

    -- Technical fields
    loaded_at_utc TEXT NOT NULL DEFAULT '',
    payload_version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_p907_transaction_date ON p907_ym_payment_report (transaction_date);
CREATE INDEX IF NOT EXISTS idx_p907_connection ON p907_ym_payment_report (connection_mp_ref);
CREATE INDEX IF NOT EXISTS idx_p907_order_id ON p907_ym_payment_report (order_id);
CREATE INDEX IF NOT EXISTS idx_p907_transaction_type ON p907_ym_payment_report (transaction_type);
CREATE INDEX IF NOT EXISTS idx_p907_shop_sku ON p907_ym_payment_report (shop_sku);
