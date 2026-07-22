-- p916: верх воронки — три показателя показов вместо одного.
-- Было: show_count (только органика a040). Стало:
--   show_free_count — бесплатные/органические показы (поисковая аналитика a040);
--   show_paid_count — платные показы (реклама a026, metrics.views).
-- «Всего показов» не хранится — считается на чтении: COALESCE(free,0)+COALESCE(paid,0).
-- Обе колонки nullable (разреженность: отсутствие данных ≠ 0).

ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN show_free_count INTEGER;
ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN show_paid_count INTEGER;

-- Существующие движения a040 (единственный источник show_count до сих пор) — это органика.
UPDATE p916_mp_sales_funnel_turnovers
SET show_free_count = show_count
WHERE show_count IS NOT NULL;

-- show_count не участвует ни в одном индексе — безопасно удалить (SQLite ≥ 3.35).
ALTER TABLE p916_mp_sales_funnel_turnovers DROP COLUMN show_count;
