-- Миграция таблицы p902_ozon_finance_realization
-- Добавляем operation_type в PRIMARY KEY и добавляем колонку is_return

-- Шаг 1: Создаем временную таблицу с новой структурой
CREATE TABLE p902_ozon_finance_realization_new (
    -- Composite Key (posting_number + sku + operation_type)
    posting_number TEXT NOT NULL,
    sku TEXT NOT NULL,

    -- Metadata
    document_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,

    -- References
    connection_mp_ref TEXT NOT NULL,
    organization_ref TEXT NOT NULL,
    posting_ref TEXT,

    -- Даты
    accrual_date TEXT NOT NULL,
    operation_date TEXT,
    delivery_date TEXT,

    -- Информация о доставке
    delivery_schema TEXT,
    delivery_region TEXT,
    delivery_city TEXT,

    -- Количество и суммы
    quantity REAL NOT NULL,
    price REAL,
    amount REAL NOT NULL,
    commission_amount REAL,
    commission_percent REAL,
    services_amount REAL,
    payout_amount REAL,

    -- Тип операции
    operation_type TEXT NOT NULL,
    operation_type_name TEXT,
    is_return INTEGER NOT NULL DEFAULT 0,

    -- Валюта
    currency_code TEXT,

    -- Технические поля
    loaded_at_utc TEXT NOT NULL,
    payload_version INTEGER NOT NULL DEFAULT 1,
    extra TEXT,

    PRIMARY KEY (posting_number, sku, operation_type)
);

-- Шаг 2: Копируем данные из старой таблицы
-- Для существующих записей устанавливаем operation_type='delivery' и is_return=0
INSERT INTO p902_ozon_finance_realization_new (
    posting_number, sku, document_type, registrator_ref,
    connection_mp_ref, organization_ref, posting_ref,
    accrual_date, operation_date, delivery_date,
    delivery_schema, delivery_region, delivery_city,
    quantity, price, amount, commission_amount, commission_percent,
    services_amount, payout_amount,
    operation_type, operation_type_name, is_return,
    currency_code, loaded_at_utc, payload_version, extra
)
SELECT
    posting_number, sku, document_type, registrator_ref,
    connection_mp_ref, organization_ref, posting_ref,
    accrual_date, operation_date, delivery_date,
    delivery_schema, delivery_region, delivery_city,
    quantity, price, amount, commission_amount, commission_percent,
    services_amount, payout_amount,
    operation_type, operation_type_name, 0 as is_return,
    currency_code, loaded_at_utc, payload_version, extra
FROM p902_ozon_finance_realization;

-- Шаг 3: Удаляем старую таблицу
DROP TABLE p902_ozon_finance_realization;

-- Шаг 4: Переименовываем новую таблицу
ALTER TABLE p902_ozon_finance_realization_new RENAME TO p902_ozon_finance_realization;

-- Шаг 5: Создаем индексы заново
CREATE INDEX IF NOT EXISTS idx_p902_accrual_date
ON p902_ozon_finance_realization (accrual_date);

CREATE INDEX IF NOT EXISTS idx_p902_posting_number
ON p902_ozon_finance_realization (posting_number);

CREATE INDEX IF NOT EXISTS idx_p902_connection_mp_ref
ON p902_ozon_finance_realization (connection_mp_ref);

CREATE INDEX IF NOT EXISTS idx_p902_posting_ref
ON p902_ozon_finance_realization (posting_ref);

-- Готово!
SELECT 'Migration completed successfully. Total records: ' || COUNT(*) as result
FROM p902_ozon_finance_realization;
