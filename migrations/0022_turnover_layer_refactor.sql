ALTER TABLE sys_journal_entries
    ADD COLUMN layer TEXT NOT NULL DEFAULT 'oper';

CREATE INDEX IF NOT EXISTS idx_sje_layer
    ON sys_journal_entries (layer);

CREATE TABLE p909_mp_order_line_turnovers_new (
    id TEXT PRIMARY KEY NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    order_key TEXT NOT NULL,
    line_key TEXT NOT NULL,
    line_event_key TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    event_date TEXT NOT NULL,
    layer TEXT NOT NULL,
    turnover_code TEXT NOT NULL,
    value_kind TEXT NOT NULL,
    agg_kind TEXT NOT NULL,
    amount REAL NOT NULL,
    nomenclature_ref TEXT,
    marketplace_product_ref TEXT,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    link_status TEXT NOT NULL,
    journal_entry_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO p909_mp_order_line_turnovers_new (
    id,
    connection_mp_ref,
    order_key,
    line_key,
    line_event_key,
    event_kind,
    event_date,
    layer,
    turnover_code,
    value_kind,
    agg_kind,
    amount,
    nomenclature_ref,
    marketplace_product_ref,
    registrator_type,
    registrator_ref,
    link_status,
    journal_entry_id,
    created_at,
    updated_at
)
SELECT
    id || ':oper',
    connection_mp_ref,
    order_key,
    line_key,
    line_event_key,
    event_kind,
    event_date,
    'oper',
    turnover_code,
    value_kind,
    agg_kind,
    amount_oper,
    nomenclature_ref,
    marketplace_product_ref,
    COALESCE(
        NULLIF(oper_source_entity, ''),
        NULLIF(order_source_entity, ''),
        'unknown'
    ),
    COALESCE(
        NULLIF(oper_source_ref, ''),
        NULLIF(order_source_ref, ''),
        id
    ),
    link_status,
    journal_entry_id,
    created_at,
    updated_at
FROM p909_mp_order_line_turnovers
WHERE amount_oper IS NOT NULL

UNION ALL

SELECT
    id || ':fact',
    connection_mp_ref,
    order_key,
    line_key,
    line_event_key,
    event_kind,
    event_date,
    'fact',
    turnover_code,
    value_kind,
    agg_kind,
    amount_fact,
    nomenclature_ref,
    marketplace_product_ref,
    COALESCE(NULLIF(fact_source_entity, ''), 'unknown'),
    COALESCE(NULLIF(fact_source_ref, ''), id),
    link_status,
    NULL,
    created_at,
    updated_at
FROM p909_mp_order_line_turnovers
WHERE amount_fact IS NOT NULL;

DROP TABLE p909_mp_order_line_turnovers;
ALTER TABLE p909_mp_order_line_turnovers_new RENAME TO p909_mp_order_line_turnovers;

CREATE INDEX idx_p909_connection_event_date
    ON p909_mp_order_line_turnovers (connection_mp_ref, event_date);
CREATE INDEX idx_p909_order_line
    ON p909_mp_order_line_turnovers (order_key, line_key);
CREATE INDEX idx_p909_turnover_code
    ON p909_mp_order_line_turnovers (turnover_code);
CREATE INDEX idx_p909_link_status
    ON p909_mp_order_line_turnovers (link_status);
CREATE INDEX idx_p909_registrator_ref
    ON p909_mp_order_line_turnovers (registrator_ref);
CREATE INDEX idx_p909_layer
    ON p909_mp_order_line_turnovers (layer);
CREATE INDEX idx_p909_journal_entry_id
    ON p909_mp_order_line_turnovers (journal_entry_id);

CREATE TABLE p910_mp_unlinked_turnovers_new (
    id TEXT PRIMARY KEY NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    turnover_date TEXT NOT NULL,
    layer TEXT NOT NULL,
    turnover_code TEXT NOT NULL,
    value_kind TEXT NOT NULL,
    agg_kind TEXT NOT NULL,
    amount REAL NOT NULL,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    nomenclature_ref TEXT,
    comment TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO p910_mp_unlinked_turnovers_new (
    id,
    connection_mp_ref,
    turnover_date,
    layer,
    turnover_code,
    value_kind,
    agg_kind,
    amount,
    registrator_type,
    registrator_ref,
    nomenclature_ref,
    comment,
    created_at,
    updated_at
)
SELECT
    id || ':oper',
    connection_mp_ref,
    turnover_date,
    'oper',
    turnover_code,
    value_kind,
    agg_kind,
    amount_oper,
    source_entity,
    source_ref,
    nomenclature_ref,
    comment,
    created_at,
    updated_at
FROM p910_mp_unlinked_turnovers
WHERE amount_oper IS NOT NULL

UNION ALL

SELECT
    id || ':fact',
    connection_mp_ref,
    turnover_date,
    'fact',
    turnover_code,
    value_kind,
    agg_kind,
    amount_fact,
    source_entity,
    source_ref,
    nomenclature_ref,
    comment,
    created_at,
    updated_at
FROM p910_mp_unlinked_turnovers
WHERE amount_fact IS NOT NULL;

DROP TABLE p910_mp_unlinked_turnovers;
ALTER TABLE p910_mp_unlinked_turnovers_new RENAME TO p910_mp_unlinked_turnovers;

CREATE INDEX idx_p910_connection_turnover_date
    ON p910_mp_unlinked_turnovers (connection_mp_ref, turnover_date);
CREATE INDEX idx_p910_turnover_code
    ON p910_mp_unlinked_turnovers (turnover_code);
CREATE INDEX idx_p910_registrator_ref
    ON p910_mp_unlinked_turnovers (registrator_ref);
CREATE INDEX idx_p910_layer
    ON p910_mp_unlinked_turnovers (layer);
