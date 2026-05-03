-- WB реклама a026: новые документы создаются как document_date + advert_id.
-- Старые документы сохраняем; после добавления колонки они остаются с advert_id = 0.

DROP INDEX IF EXISTS idx_a026_wb_advert_daily_connection_date;

ALTER TABLE a026_wb_advert_daily ADD COLUMN advert_id INTEGER NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX IF NOT EXISTS idx_a026_wb_advert_daily_connection_date_advert
    ON a026_wb_advert_daily(connection_id, document_date, advert_id)
    WHERE is_deleted = 0;
