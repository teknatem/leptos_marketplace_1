-- p916_mp_sales_funnel_turnovers: универсальная воронка продаж МП (WB; далее YM/OZON).
-- Проекция-накопитель «движения-обороты»: каждый регистратор пишет свои знаковые строки
-- при проведении/импорте (delete-by-registrator + insert), агрегация SUM на чтении —
-- по образцу p914_mp_finance_turnovers / p915_mp_order_events.
--
-- Две стадии воронки в одной таблице (различаются полем stage и набором метрик):
--   marketing   (стадия 1, источник a036): показы(резерв)/переходы/корзина/отложенные/
--               заказы_воронки — верх воронки, cohort_date = event_date = день воронки.
--   fulfillment (стадия 2, источники a015/a012): заказы/отмены/выкупы/возвраты —
--               cohort_date = дата заказа (когорта), event_date = дата транзакции события.
--
-- Оптимизация размера: «широкие» строки (все метрики стадии — колонками одной строки),
-- разреженность (пустые строки не пишутся), без денормализации имён (джойн a004/a007 на чтении),
-- без хранения производных конверсий (считаются в запросе).

CREATE TABLE p916_mp_sales_funnel_turnovers (
    id TEXT NOT NULL PRIMARY KEY,
    stage TEXT NOT NULL,                     -- 'marketing' | 'fulfillment'
    cohort_date TEXT NOT NULL,               -- ось когорты (дата заказа; стадия1 — день воронки), YYYY-MM-DD
    event_date TEXT NOT NULL,                -- ось потока (дата транзакции; стадия1 = cohort_date), YYYY-MM-DD
    connection_mp_ref TEXT NOT NULL,
    marketplace_product_ref TEXT,            -- a007 uuid — универсальная идентичность товара; nullable
    nomenclature_ref TEXT,                   -- a004 uuid — для отчётности/джойна; nullable
    nm_id INTEGER,                           -- native WB nmId (удобство/мост для WB); nullable
    registrator_type TEXT NOT NULL,          -- 'a036_wb_sales_funnel_daily' | 'a015_wb_orders' | 'a012_wb_sales'
    registrator_ref TEXT NOT NULL,           -- id документа-источника (идемпотентность)

    -- стадия 1 (маркетинговая воронка):
    show_count INTEGER,                      -- резерв под показы (органика/поисковая аналитика), nullable
    open_count INTEGER NOT NULL DEFAULT 0,   -- переходы в карточку (openCard)
    cart_count INTEGER NOT NULL DEFAULT 0,
    wishlist_count INTEGER NOT NULL DEFAULT 0,
    funnel_order_count INTEGER NOT NULL DEFAULT 0,  -- заказы «глазами воронки» a036 (≠ order_count!)
    funnel_order_sum REAL NOT NULL DEFAULT 0,

    -- стадия 2 (fulfillment/когорта):
    order_count INTEGER NOT NULL DEFAULT 0,  -- фактические заказы (a015)
    order_sum REAL NOT NULL DEFAULT 0,
    cancel_count INTEGER NOT NULL DEFAULT 0,
    cancel_sum REAL NOT NULL DEFAULT 0,
    buyout_count INTEGER NOT NULL DEFAULT 0, -- выкупы (a012)
    buyout_sum REAL NOT NULL DEFAULT 0,
    return_count INTEGER NOT NULL DEFAULT 0, -- возвраты покупателя (a012)
    return_sum REAL NOT NULL DEFAULT 0,

    created_at_msk TEXT NOT NULL,
    updated_at_msk TEXT NOT NULL
);

-- Когортная ось (дата заказа × товар × кабинет) — основной разрез отчётности.
CREATE INDEX idx_p916_cohort ON p916_mp_sales_funnel_turnovers(connection_mp_ref, cohort_date, marketplace_product_ref);
-- Потоковая/кассовая ось (дата транзакции).
CREATE INDEX idx_p916_event ON p916_mp_sales_funnel_turnovers(connection_mp_ref, event_date);
-- Идемпотентность проведения (delete-by-registrator).
CREATE INDEX idx_p916_registrator ON p916_mp_sales_funnel_turnovers(registrator_type, registrator_ref);
-- WB-мост по nm_id для стыковки стадии 1 (a036) и стадии 2 (a015/a012).
CREATE INDEX idx_p916_nm_id ON p916_mp_sales_funnel_turnovers(connection_mp_ref, nm_id, cohort_date);
