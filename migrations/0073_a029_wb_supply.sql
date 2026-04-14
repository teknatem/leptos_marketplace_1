CREATE TABLE IF NOT EXISTS a029_wb_supply (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,
    supply_id TEXT NOT NULL,
    supply_name TEXT,
    is_done INTEGER NOT NULL DEFAULT 0,
    is_b2b INTEGER NOT NULL DEFAULT 0,
    created_at_wb TEXT,
    closed_at_wb TEXT,
    scan_dt TEXT,
    cargo_type INTEGER,
    connection_id TEXT NOT NULL DEFAULT '',
    organization_id TEXT NOT NULL DEFAULT '',
    marketplace_id TEXT NOT NULL DEFAULT '',
    info_json TEXT NOT NULL DEFAULT '{}',
    supply_orders_json TEXT NOT NULL DEFAULT '[]',
    source_meta_json TEXT NOT NULL DEFAULT '{}',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a029_wb_supply_supply_id_connection
    ON a029_wb_supply(supply_id, connection_id)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a029_wb_supply_created_at_wb
    ON a029_wb_supply(created_at_wb);

CREATE INDEX IF NOT EXISTS idx_a029_wb_supply_closed_at_wb
    ON a029_wb_supply(closed_at_wb);

CREATE INDEX IF NOT EXISTS idx_a029_wb_supply_connection_id
    ON a029_wb_supply(connection_id);

CREATE INDEX IF NOT EXISTS idx_a029_wb_supply_is_done
    ON a029_wb_supply(is_done);
