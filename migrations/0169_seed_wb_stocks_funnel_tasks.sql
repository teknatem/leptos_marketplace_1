-- Seed: обновление данных WB по остаткам (a037/task020) и воронке продаж (a036/task023)
-- дважды в день по обоим WB-кабинетам.
--
-- ВРЕМЯ: планировщик считает cron в UTC (worker.rs использует Utc::now()), МСК = UTC+3.
--   '0 0 3,15 * * *'  → 06:00 и 18:00 МСК — остатки (task020)
--   '0 30 3,15 * * *' → 06:30 и 18:30 МСК — воронка (task023)
-- Воронка смещена на 30 минут намеренно: task020 и task023 бьют в один эндпоинт
-- sales-funnel/products одним API-ключом кабинета с лимитом 3 запроса/мин, а троттлинг
-- живёт внутри каждого ImportExecutor отдельно — одновременный старт даст 429.
-- Разные кабинеты друг другу не мешают: у них разные ключи и независимые лимиты.
--
-- Оба источника forward-only: WB не отдаёт ни остатки задним числом, ни воронку старше
-- ~недели. Поэтому задания создаются СРАЗУ ВКЛЮЧЁННЫМИ с реальными connection_id.
--
-- ПЕРВЫЙ ЗАПУСК: воркер считает next_run_at = NULL как «запустить немедленно» (worker.rs:88).
-- Остаткам это и нужно — они не собирались ни разу, снимок делается только вперёд.
-- А вот воронке ставим next_run_at = +40 минут: иначе на первом же тике task020 и task023
-- по одному кабинету стартуют одновременно и подерутся за лимит 3 запроса/мин (cron-сдвиг
-- на 30 минут защищает только установившийся режим, но не первый запуск).
-- После первого прогона next_run_at пересчитывается из cron и расписание встаёт в график.

-- 1. Существующая строка task020 (сеялась плейсхолдером и выключенной в 0160) →
--    кабинет WB - SANSTAR, 2 раза в день, включена.
--    Guard по REPLACE_WITH: если строку уже настроили руками, ничего не трогаем.
UPDATE sys_tasks
SET code = 'task020-wb-product-snapshot_SANSTAR',
    description = 'WB Снимки товаров — остатки и рейтинги (WB - SANSTAR, 06:00 и 18:00 МСК).',
    schedule_cron = '0 0 3,15 * * *',
    config_json = '{"connection_id":"1386a311-1e26-4676-b696-8d577a119eec","window_days":7}',
    is_enabled = 1,
    next_run_at = NULL,
    updated_at = datetime('now')
WHERE id = 'a1b2c3d4-e5f6-7890-abcd-ef1234567820'
  AND config_json LIKE '%REPLACE_WITH%';

-- 2. Вторая строка task020 — кабинет WB2 - CTC.
INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, created_at, updated_at, is_deleted
) VALUES (
    'c0200020-0000-4020-b020-000000000020',
    'task020-wb-product-snapshot_CTC',
    'WB Снимки товаров — остатки и рейтинги (WB2 - CTC, 06:00 и 18:00 МСК).',
    'task020_wb_product_snapshot',
    '0 0 3,15 * * *',
    '{"connection_id":"42e29532-72b1-4f38-be6e-38c331c61fe6","window_days":7}',
    1,
    datetime('now'),
    datetime('now'),
    0
);

-- 3. Воронка продаж — по строке на кабинет.
INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, next_run_at, created_at, updated_at, is_deleted
) VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567823',
    'task023-wb-sales-funnel_SANSTAR',
    'WB Воронка продаж по дням (WB - SANSTAR, 06:30 и 18:30 МСК). WB хранит ~7 дней.',
    'task023_wb_sales_funnel_daily',
    '0 30 3,15 * * *',
    '{"connection_id":"1386a311-1e26-4676-b696-8d577a119eec","window_days":7}',
    1,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now', '+40 minutes'),
    datetime('now'),
    datetime('now'),
    0
);

INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, next_run_at, created_at, updated_at, is_deleted
) VALUES (
    'c0230023-0000-4023-b023-000000000023',
    'task023-wb-sales-funnel_CTC',
    'WB Воронка продаж по дням (WB2 - CTC, 06:30 и 18:30 МСК). WB хранит ~7 дней.',
    'task023_wb_sales_funnel_daily',
    '0 30 3,15 * * *',
    '{"connection_id":"42e29532-72b1-4f38-be6e-38c331c61fe6","window_days":7}',
    1,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now', '+40 minutes'),
    datetime('now'),
    datetime('now'),
    0
);
