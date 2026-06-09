CREATE TABLE IF NOT EXISTS a035_ym_settlement_recon (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,
    bank_order_id INTEGER NOT NULL DEFAULT 0,
    bank_order_date TEXT NOT NULL DEFAULT '',
    connection_id TEXT NOT NULL DEFAULT '',
    organization_id TEXT NOT NULL DEFAULT '',
    marketplace_id TEXT NOT NULL DEFAULT '',
    period_from TEXT NOT NULL DEFAULT '',
    period_to TEXT NOT NULL DEFAULT '',
    bank_sum REAL NOT NULL DEFAULT 0,
    theoretical_sum REAL NOT NULL DEFAULT 0,
    deviation REAL NOT NULL DEFAULT 0,
    abs_deviation REAL NOT NULL DEFAULT 0,
    header_json TEXT NOT NULL DEFAULT '{}',
    totals_json TEXT NOT NULL DEFAULT '{}',
    lines_json TEXT NOT NULL DEFAULT '[]',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

-- Один документ = один банковский ордер по кабинету (стабильный детерминированный id).
CREATE UNIQUE INDEX IF NOT EXISTS idx_a035_ym_settlement_recon_conn_order
    ON a035_ym_settlement_recon(connection_id, bank_order_id)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a035_ym_settlement_recon_bank_order_date
    ON a035_ym_settlement_recon(bank_order_date);

CREATE INDEX IF NOT EXISTS idx_a035_ym_settlement_recon_connection
    ON a035_ym_settlement_recon(connection_id);
