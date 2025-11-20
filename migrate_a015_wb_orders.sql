-- Миграция для создания таблицы a015_wb_orders (Wildberries Orders)
-- Дата: 2025-11-18
-- Агрегат: a015_wb_orders

-- ==================================================================
-- Таблица: a015_wb_orders
-- Описание: Хранит заказы из Wildberries API
-- ==================================================================

CREATE TABLE IF NOT EXISTS a015_wb_orders (
    -- Primary Key
    id TEXT PRIMARY KEY NOT NULL,
    
    -- Основные поля
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    comment TEXT,
    
    -- Номер документа (srid - уникальный идентификатор заказа)
    document_no TEXT NOT NULL,
    
    -- Новые поля из WB API
    document_date TEXT,                  -- date из API (основная дата заказа)
    g_number TEXT,                       -- gNumber из API
    spp REAL,                            -- spp (согласованная скидка продавца)
    is_cancel INTEGER,                   -- isCancel (флаг отмены)
    cancel_date TEXT,                    -- cancelDate (дата отмены)
    
    -- JSON поля для структурированных данных
    header_json TEXT NOT NULL,           -- WbOrdersHeader
    line_json TEXT NOT NULL,             -- WbOrdersLine
    state_json TEXT NOT NULL,            -- WbOrdersState
    warehouse_json TEXT NOT NULL,        -- WbOrdersWarehouse
    geography_json TEXT NOT NULL,        -- WbOrdersGeography
    source_meta_json TEXT NOT NULL,      -- WbOrdersSourceMeta
    
    -- Ссылки
    marketplace_product_ref TEXT,        -- Ссылка на a007_marketplace_product
    nomenclature_ref TEXT,               -- Ссылка на a004_nomenclature
    
    -- Флаги
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    
    -- Audit поля
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

-- ==================================================================
-- Индексы
-- ==================================================================

-- Уникальный индекс по document_no (srid должен быть уникален)
CREATE UNIQUE INDEX IF NOT EXISTS idx_a015_wb_orders_document_no
    ON a015_wb_orders(document_no);

-- Индекс для soft delete
CREATE INDEX IF NOT EXISTS idx_a015_wb_orders_deleted
    ON a015_wb_orders(is_deleted);

-- Индекс по дате создания
CREATE INDEX IF NOT EXISTS idx_a015_wb_orders_created_at
    ON a015_wb_orders(created_at);

-- Индекс по posted статусу
CREATE INDEX IF NOT EXISTS idx_a015_wb_orders_posted
    ON a015_wb_orders(is_posted);

-- Индекс по ссылке на marketplace_product
CREATE INDEX IF NOT EXISTS idx_a015_wb_orders_mp_ref
    ON a015_wb_orders(marketplace_product_ref);

-- Индекс по ссылке на nomenclature
CREATE INDEX IF NOT EXISTS idx_a015_wb_orders_nom_ref
    ON a015_wb_orders(nomenclature_ref);

-- Индекс по дате документа (для быстрой фильтрации)
CREATE INDEX IF NOT EXISTS idx_a015_wb_orders_document_date
    ON a015_wb_orders(document_date);

-- ==================================================================
-- Комментарии
-- ==================================================================

-- Таблица для хранения заказов из Wildberries API
-- 
-- Структура данных:
-- - header_json: connection_id, organization_id, marketplace_id
-- - line_json: supplier_article, nm_id, barcode, category, subject, brand,
--              tech_size, qty, total_price, discount_percent, spp,
--              finished_price, price_with_disc
-- - state_json: order_dt, last_change_dt, is_cancel, cancel_dt,
--               is_supply, is_realization
-- - warehouse_json: warehouse_name, warehouse_type
-- - geography_json: country_name, oblast_okrug_name, region_name
-- - source_meta_json: income_id, sticker, g_number, raw_payload_ref,
--                     fetched_at, document_version
--
-- API Endpoint: GET /api/v1/supplier/orders
-- Уникальность: document_no (srid из WB API)

