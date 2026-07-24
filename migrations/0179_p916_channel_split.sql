-- p916: канальный сплит воронки d406 (Все / Платные / Бесплатные).
-- Верх воронки (переходы/корзина) для платного канала — из собственных счётчиков рекламы a026:
--   paid_open_count — платные переходы (a026 metrics.clicks);
--   paid_cart_count — платная корзина (a026 metrics.atbs).
-- Низ воронки (заказы/выкупы/отмены/возвраты) делится на чтении по вхождению srid заказа в
-- атрибуцию рекламы p913 — для этого fulfillment-строки несут order_key (srid = document_no).
-- Все колонки nullable (N/A ≠ 0; order_key — только на fulfillment-строках).

ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN order_key TEXT;
ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN paid_open_count INTEGER;
ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN paid_cart_count INTEGER;

-- Джойн к p913 на чтении идёт по order_key; индекс ускоряет агрегацию воронки.
CREATE INDEX IF NOT EXISTS idx_p916_order_key
    ON p916_mp_sales_funnel_turnovers (order_key);
