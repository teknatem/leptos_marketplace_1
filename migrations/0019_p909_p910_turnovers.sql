CREATE TABLE IF NOT EXISTS p909_mp_order_line_turnovers (
    id TEXT PRIMARY KEY NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    order_key TEXT NOT NULL,
    line_key TEXT NOT NULL,
    line_event_key TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    event_date TEXT NOT NULL,
    turnover_code TEXT NOT NULL,
    value_kind TEXT NOT NULL,
    agg_kind TEXT NOT NULL,
    amount_oper REAL,
    amount_fact REAL,
    nomenclature_ref TEXT,
    marketplace_product_ref TEXT,
    order_source_entity TEXT,
    order_source_ref TEXT,
    oper_source_entity TEXT,
    oper_source_ref TEXT,
    fact_source_entity TEXT,
    fact_source_ref TEXT,
    link_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_p909_connection_event_date
    ON p909_mp_order_line_turnovers (connection_mp_ref, event_date);
CREATE INDEX IF NOT EXISTS idx_p909_order_line
    ON p909_mp_order_line_turnovers (order_key, line_key);
CREATE INDEX IF NOT EXISTS idx_p909_turnover_code
    ON p909_mp_order_line_turnovers (turnover_code);
CREATE INDEX IF NOT EXISTS idx_p909_link_status
    ON p909_mp_order_line_turnovers (link_status);
CREATE INDEX IF NOT EXISTS idx_p909_oper_source_ref
    ON p909_mp_order_line_turnovers (oper_source_ref);
CREATE INDEX IF NOT EXISTS idx_p909_order_source_ref
    ON p909_mp_order_line_turnovers (order_source_ref);
CREATE INDEX IF NOT EXISTS idx_p909_fact_source_ref
    ON p909_mp_order_line_turnovers (fact_source_ref);

CREATE TABLE IF NOT EXISTS p910_mp_unlinked_turnovers (
    id TEXT PRIMARY KEY NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    turnover_date TEXT NOT NULL,
    turnover_code TEXT NOT NULL,
    value_kind TEXT NOT NULL,
    agg_kind TEXT NOT NULL,
    amount_oper REAL,
    amount_fact REAL,
    source_entity TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    source_row_key TEXT NOT NULL,
    nomenclature_ref TEXT,
    comment TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_p910_connection_turnover_date
    ON p910_mp_unlinked_turnovers (connection_mp_ref, turnover_date);
CREATE INDEX IF NOT EXISTS idx_p910_turnover_code
    ON p910_mp_unlinked_turnovers (turnover_code);
CREATE INDEX IF NOT EXISTS idx_p910_source_row
    ON p910_mp_unlinked_turnovers (source_entity, source_row_key);
