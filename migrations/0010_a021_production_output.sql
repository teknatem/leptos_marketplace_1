-- Агрегат a021: Выпуск продукции (из ERP/1С)
CREATE TABLE IF NOT EXISTS a021_production_output (
    id            TEXT PRIMARY KEY,        -- UUID из 1С (поле "id" в JSON ответе)
    code          TEXT NOT NULL DEFAULT '', -- Номер документа (document_no)
    description   TEXT NOT NULL DEFAULT '', -- Наименование продукта
    comment       TEXT,
    document_no   TEXT NOT NULL,           -- Номер документа, напр. "Ф-00000084"
    document_date TEXT NOT NULL,           -- Дата производства "YYYY-MM-DD"
    article       TEXT NOT NULL DEFAULT '', -- Артикул (для поиска в a004_nomenclature)
    count         INTEGER NOT NULL DEFAULT 0, -- Количество произведённых единиц
    amount        REAL NOT NULL DEFAULT 0, -- Сумма себестоимости итого
    cost_of_production REAL,               -- Себестоимость на 1 шт (amount / count)
    nomenclature_ref   TEXT,               -- UUID из a004_nomenclature (опционально)
    connection_id      TEXT NOT NULL DEFAULT '', -- UUID подключения a001_connection_1c
    fetched_at         TEXT NOT NULL,      -- Дата и время загрузки (ISO 8601)
    is_deleted    INTEGER NOT NULL DEFAULT 0,
    is_posted     INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT,
    updated_at    TEXT,
    version       INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a021_document_date ON a021_production_output(document_date);
CREATE INDEX IF NOT EXISTS idx_a021_article ON a021_production_output(article);
CREATE INDEX IF NOT EXISTS idx_a021_document_no ON a021_production_output(document_no);
CREATE INDEX IF NOT EXISTS idx_a021_nomenclature_ref ON a021_production_output(nomenclature_ref);
