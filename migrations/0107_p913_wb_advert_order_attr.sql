CREATE TABLE p913_wb_advert_order_attr (
    id TEXT NOT NULL PRIMARY KEY,
    connection_mp_ref TEXT NOT NULL,
    entry_date TEXT NOT NULL,
    turnover_code TEXT NOT NULL,
    amount REAL NOT NULL,
    nomenclature_ref TEXT,
    wb_advert_campaign_ref TEXT NOT NULL,
    order_key TEXT NOT NULL,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    general_ledger_ref TEXT,
    is_problem INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_p913_registrator ON p913_wb_advert_order_attr(registrator_ref);
CREATE INDEX idx_p913_order_key ON p913_wb_advert_order_attr(order_key);
CREATE INDEX idx_p913_entry_date ON p913_wb_advert_order_attr(entry_date);
CREATE INDEX idx_p913_campaign ON p913_wb_advert_order_attr(wb_advert_campaign_ref);
