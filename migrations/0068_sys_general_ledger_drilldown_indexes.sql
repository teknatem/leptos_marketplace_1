-- Speeds up the dominant dv004 / GL drilldown predicates:
-- turnover_code = ?
-- layer = ?
-- entry_date between ? and ?
-- optionally connection_mp_ref = ?

CREATE INDEX IF NOT EXISTS idx_sgl_turnover_layer_entry_date
    ON sys_general_ledger (turnover_code, layer, entry_date);

CREATE INDEX IF NOT EXISTS idx_sgl_turnover_layer_connection_entry_date
    ON sys_general_ledger (turnover_code, layer, connection_mp_ref, entry_date);
