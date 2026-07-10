-- Seed: ежедневный снимок остатков и рейтингов товаров WB (task020)
-- ВНИМАНИЕ: Перед включением замените connection_id на реальный UUID WB-кабинета
--           из справочника a006_connection_mp. Задание создаётся отключённым (is_enabled = 0).
-- Снимок снимается только вперёд (WB отдаёт остатки/рейтинги как текущее состояние).
-- Для нескольких кабинетов создайте отдельную строку sys_tasks на каждый.

INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567820',
    'task020-wb-product-snapshot',
    'WB Снимки товаров — остатки и рейтинги (раз в день). Замените connection_id на UUID WB-кабинета.',
    'task020_wb_product_snapshot',
    '0 0 3 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","window_days":7}',
    0, datetime('now'), datetime('now'), 0
);
