-- Унификация p911_wb_advert_by_items с p913_wb_advert_order_attr.
-- Удаляем мёртвые поля layer/value_kind/agg_kind (все строки имеют одинаковые значения),
-- добавляем wb_advert_campaign_code (backfill через JOIN с a026_wb_advert_daily.advert_id).

CREATE TABLE p911_wb_advert_by_items_new (
    id TEXT PRIMARY KEY NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    entry_date TEXT NOT NULL,
    turnover_code TEXT NOT NULL,
    amount REAL NOT NULL,
    nomenclature_ref TEXT,
    wb_advert_campaign_code TEXT NOT NULL DEFAULT '',
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    general_ledger_ref TEXT,
    is_problem INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO p911_wb_advert_by_items_new (
    id,
    connection_mp_ref,
    entry_date,
    turnover_code,
    amount,
    nomenclature_ref,
    wb_advert_campaign_code,
    registrator_type,
    registrator_ref,
    general_ledger_ref,
    is_problem,
    created_at,
    updated_at
)
SELECT
    p.id,
    p.connection_mp_ref,
    p.entry_date,
    p.turnover_code,
    p.amount,
    p.nomenclature_ref,
    COALESCE(CAST(a.advert_id AS TEXT), ''),
    p.registrator_type,
    p.registrator_ref,
    p.general_ledger_ref,
    p.is_problem,
    p.created_at,
    p.updated_at
FROM p911_wb_advert_by_items p
LEFT JOIN a026_wb_advert_daily a ON a.id = p.registrator_ref;

DROP TABLE p911_wb_advert_by_items;
ALTER TABLE p911_wb_advert_by_items_new RENAME TO p911_wb_advert_by_items;

CREATE INDEX idx_p911_connection_entry_date
    ON p911_wb_advert_by_items (connection_mp_ref, entry_date);
CREATE INDEX idx_p911_nomenclature_ref
    ON p911_wb_advert_by_items (nomenclature_ref);
CREATE INDEX idx_p911_registrator_ref
    ON p911_wb_advert_by_items (registrator_ref);
CREATE INDEX idx_p911_general_ledger_ref
    ON p911_wb_advert_by_items (general_ledger_ref);
CREATE INDEX idx_p911_is_problem
    ON p911_wb_advert_by_items (is_problem);
CREATE INDEX idx_p911_wb_advert_campaign_code
    ON p911_wb_advert_by_items (wb_advert_campaign_code);
