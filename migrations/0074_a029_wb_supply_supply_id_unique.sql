DROP INDEX IF EXISTS idx_a029_wb_supply_supply_id_connection;

CREATE UNIQUE INDEX IF NOT EXISTS idx_a029_wb_supply_supply_id
    ON a029_wb_supply(supply_id);
