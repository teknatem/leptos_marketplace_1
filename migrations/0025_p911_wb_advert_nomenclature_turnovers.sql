CREATE TABLE IF NOT EXISTS p911_wb_advert_nomenclature_turnovers (
    id TEXT PRIMARY KEY NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    turnover_date TEXT NOT NULL,
    layer TEXT NOT NULL,
    turnover_code TEXT NOT NULL,
    value_kind TEXT NOT NULL,
    agg_kind TEXT NOT NULL,
    amount REAL NOT NULL,
    nomenclature_ref TEXT,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    journal_entry_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_p911_turnover_date
    ON p911_wb_advert_nomenclature_turnovers (turnover_date);
CREATE INDEX IF NOT EXISTS idx_p911_connection_mp_ref
    ON p911_wb_advert_nomenclature_turnovers (connection_mp_ref);
CREATE INDEX IF NOT EXISTS idx_p911_nomenclature_ref
    ON p911_wb_advert_nomenclature_turnovers (nomenclature_ref);
CREATE INDEX IF NOT EXISTS idx_p911_registrator_ref
    ON p911_wb_advert_nomenclature_turnovers (registrator_ref);
CREATE INDEX IF NOT EXISTS idx_p911_journal_entry_id
    ON p911_wb_advert_nomenclature_turnovers (journal_entry_id);
