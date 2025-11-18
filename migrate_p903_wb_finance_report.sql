-- ============================================================
-- Migration: Create P903 WB Finance Report Table
-- Description: Таблица для хранения финансовых отчетов Wildberries
-- Date: 2025-11-17
-- ============================================================

-- Проверяем существование таблицы и создаем если не существует
CREATE TABLE IF NOT EXISTS p903_wb_finance_report (
    -- Composite Primary Key
    rr_dt TEXT NOT NULL,              -- Дата строки финансового отчёта (расчетный день)
    rrd_id INTEGER NOT NULL,          -- Внутренний ID строки отчета

    -- Metadata
    connection_mp_ref TEXT NOT NULL,  -- Ссылка на подключение к маркетплейсу (для предотвращения дублей)
    organization_ref TEXT NOT NULL,   -- Ссылка на организацию

    -- Main Fields (22 specified fields)
    acquiring_fee REAL,               -- Комиссия за эквайринг (приём оплаты банковской картой)
    acquiring_percent REAL,           -- Процент комиссии за эквайринг
    additional_payment REAL,          -- Дополнительные (корректирующие) выплаты продавцу
    bonus_type_name TEXT,             -- Тип бонуса или штрафа, применённого к строке отчёта
    commission_percent REAL,          -- Процент комиссии Wildberries с продажи
    delivery_amount REAL,             -- Сумма, уплаченная покупателем за доставку
    delivery_rub REAL,                -- Стоимость доставки на стороне продавца
    nm_id INTEGER,                    -- Внутренний идентификатор товара Wildberries (артикул)
    penalty REAL,                     -- Сумма штрафа, удержанного с продавца
    ppvz_vw REAL,                     -- Сумма услуги по возврату денежных средств (вычет за возвраты, логистика)
    ppvz_vw_nds REAL,                 -- НДС по услуге возврата денежных средств
    ppvz_sales_commission REAL,       -- Комиссия WB за продажу
    quantity INTEGER,                 -- Количество товаров
    rebill_logistic_cost REAL,        -- Скорректированные расходы на логистику
    retail_amount REAL,               -- Общая сумма продажи (стоимость всех единиц товара)
    retail_price REAL,                -- Розничная цена за единицу товара (без скидок)
    retail_price_withdisc_rub REAL,   -- Цена продажи с учетом всех скидок и промо-акций (итоговая цена для клиента)
    return_amount REAL,               -- Сумма возврата за возвращённые товары
    sa_name TEXT,                     -- Артикул продавца
    storage_fee REAL,                 -- Плата за хранение товаров на складе Wildberries
    subject_name TEXT,                -- Категория или название товара (тип предмета, например, "Мини-печи")
    supplier_oper_name TEXT,          -- Тип операции по заказу (Продажа, Возврат, Итд)
    cashback_amount REAL,             -- Сумма кэшбэка
    ppvz_for_pay REAL,                -- К перечислению за товар
    ppvz_kvw_prc REAL,                -- Процент комиссии
    ppvz_kvw_prc_base REAL,           -- Базовый процент комиссии
    srv_dbs INTEGER,                  -- Доставка силами продавца (0/1)

    -- Technical fields
    loaded_at_utc TEXT NOT NULL,      -- Дата и время загрузки данных (UTC)
    payload_version INTEGER NOT NULL DEFAULT 1,  -- Версия структуры данных
    extra TEXT,                       -- Full JSON from API (полная копия данных строки из API)

    PRIMARY KEY (rr_dt, rrd_id)
);

-- Create indexes for fast search
CREATE INDEX IF NOT EXISTS idx_p903_rr_dt
ON p903_wb_finance_report (rr_dt);

CREATE INDEX IF NOT EXISTS idx_p903_nm_id
ON p903_wb_finance_report (nm_id);

CREATE INDEX IF NOT EXISTS idx_p903_connection_mp_ref
ON p903_wb_finance_report (connection_mp_ref);

-- ============================================================
-- Verification queries (для проверки созданной таблицы)
-- ============================================================

-- Проверить структуру таблицы
-- PRAGMA table_info(p903_wb_finance_report);

-- Проверить индексы
-- SELECT * FROM sqlite_master WHERE type='index' AND tbl_name='p903_wb_finance_report';

-- Проверить количество записей
-- SELECT COUNT(*) as total_records FROM p903_wb_finance_report;

-- Проверить данные по дате
-- SELECT 
--     rr_dt,
--     COUNT(*) as records_count,
--     SUM(quantity) as total_quantity,
--     SUM(retail_amount) as total_amount
-- FROM p903_wb_finance_report
-- GROUP BY rr_dt
-- ORDER BY rr_dt DESC
-- LIMIT 10;

