-- Переименование wb_advert_campaign_ref → wb_advert_campaign_code в p913,
-- т.к. поле хранит advert_id (целое число WB), а не UUID. Суффикс _ref зарезервирован для UUID.
-- Также добавляем недостающие индексы по general_ledger_ref и (connection_mp_ref, entry_date).

ALTER TABLE p913_wb_advert_order_attr
    RENAME COLUMN wb_advert_campaign_ref TO wb_advert_campaign_code;

DROP INDEX IF EXISTS idx_p913_campaign;
CREATE INDEX idx_p913_wb_advert_campaign_code
    ON p913_wb_advert_order_attr (wb_advert_campaign_code);

CREATE INDEX IF NOT EXISTS idx_p913_general_ledger_ref
    ON p913_wb_advert_order_attr (general_ledger_ref);
CREATE INDEX IF NOT EXISTS idx_p913_connection_entry_date
    ON p913_wb_advert_order_attr (connection_mp_ref, entry_date);
