-- Performance indexes for dv003_mp_order_line_turnovers queries.
-- dv003 always filters layer='oper' and entry_date range.

CREATE INDEX IF NOT EXISTS idx_p909_layer_connection_entry_date
    ON p909_mp_order_line_turnovers (layer, connection_mp_ref, entry_date);

CREATE INDEX IF NOT EXISTS idx_p909_layer_turnover_entry_date
    ON p909_mp_order_line_turnovers (layer, turnover_code, entry_date);
