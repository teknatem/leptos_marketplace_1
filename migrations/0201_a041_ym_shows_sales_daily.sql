-- a041 — дневная воронка Яндекс.Маркета из отчёта «Аналитика продаж» (shows-sales).
--
-- Единственный источник показов и кликов YM: в orders-API (a013) их нет. Аналог a036
-- у Wildberries; питает стадию marketing проекции p916.
--
-- Все метрики nullable: YM не гарантирует полный набор колонок на всех тарифах, а
-- глубина истории ограничена подпиской (90 дней без неё, 400 на «Медиум»). NULL —
-- «данных не было» (N/A); нулевые и отсутствующие показы дают разные конверсии,
-- поэтому DEFAULT 0 здесь недопустим.

CREATE TABLE IF NOT EXISTS a041_ym_shows_sales_daily (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    description TEXT NOT NULL,
    comment TEXT,
    document_no TEXT NOT NULL,
    document_date TEXT NOT NULL,
    connection_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    marketplace_id TEXT NOT NULL,
    campaign_id TEXT,
    lines_count INTEGER NOT NULL DEFAULT 0,
    total_shows INTEGER,
    total_clicks INTEGER,
    total_to_cart INTEGER,
    total_order_items INTEGER,
    total_delivered_count INTEGER,
    total_canceled_count INTEGER,
    total_returned_count INTEGER,
    header_json TEXT NOT NULL,
    totals_json TEXT NOT NULL,
    lines_json TEXT NOT NULL,
    source_meta_json TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

-- Документ уникален по (кабинет, дата) — id детерминирован от этой же пары.
CREATE UNIQUE INDEX IF NOT EXISTS idx_a041_connection_date
    ON a041_ym_shows_sales_daily (connection_id, document_date);
CREATE INDEX IF NOT EXISTS idx_a041_document_date
    ON a041_ym_shows_sales_daily (document_date);

-- p916: общие показы маркетплейса без деления на платные/органические.
-- Заполняет YM; у WB остаётся NULL — там общего счётчика показов нет, есть только
-- платный show_paid_count. Отдельная колонка (а не переиспользование show_free_count),
-- чтобы разрезы «платные/органические» и «всего» не смешивались.
ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN total_impressions INTEGER;
