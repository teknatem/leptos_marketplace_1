CREATE TABLE IF NOT EXISTS p912_nomenclature_costs (
    id TEXT PRIMARY KEY NOT NULL,
    period TEXT NOT NULL,
    nomenclature_ref TEXT NOT NULL,
    cost REAL NOT NULL DEFAULT 0,
    quantity REAL,
    amount REAL,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    line_no INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_p912_period ON p912_nomenclature_costs(period);
CREATE INDEX IF NOT EXISTS idx_p912_nomenclature_ref ON p912_nomenclature_costs(nomenclature_ref);
CREATE INDEX IF NOT EXISTS idx_p912_registrator ON p912_nomenclature_costs(registrator_type, registrator_ref);
CREATE UNIQUE INDEX IF NOT EXISTS idx_p912_registrator_line
    ON p912_nomenclature_costs(registrator_type, registrator_ref, line_no);
