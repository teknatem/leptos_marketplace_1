-- Агрегат a023: Приобретение товаров и услуг (из 1С УТ 11 OData)
CREATE TABLE IF NOT EXISTS a023_purchase_of_goods (
    id              TEXT PRIMARY KEY,           -- UUID документа из 1С (Ref_Key)
    code            TEXT NOT NULL DEFAULT '',   -- Номер документа (дублируется в code для единообразия)
    description     TEXT NOT NULL DEFAULT '',   -- Описание (контрагент + дата)
    comment         TEXT,
    document_no     TEXT NOT NULL,              -- Номер документа, напр. "ПОСТ-000001"
    document_date   TEXT NOT NULL,              -- Дата документа "YYYY-MM-DD"
    counterparty_key TEXT NOT NULL DEFAULT '',  -- UUID контрагента из 1С (a003_counterparty)
    lines_json      TEXT,                       -- JSON массив строк Товары
    connection_id   TEXT NOT NULL DEFAULT '',   -- UUID подключения a001_connection_1c
    fetched_at      TEXT NOT NULL,              -- Дата и время загрузки (ISO 8601)
    is_deleted      INTEGER NOT NULL DEFAULT 0,
    is_posted       INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT,
    updated_at      TEXT,
    version         INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a023_document_date    ON a023_purchase_of_goods(document_date);
CREATE INDEX IF NOT EXISTS idx_a023_document_no      ON a023_purchase_of_goods(document_no);
CREATE INDEX IF NOT EXISTS idx_a023_counterparty_key ON a023_purchase_of_goods(counterparty_key);
