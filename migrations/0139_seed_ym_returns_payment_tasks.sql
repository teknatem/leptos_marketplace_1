-- Seed: атомарные YM-задания — возвраты (task018) и отчёт по платежам (task019).
-- ВНИМАНИЕ: Перед включением замените connection_id на реальный UUID YM-кабинета
--           из справочника a006_connection_mp. Задания создаются отключёнными (is_enabled = 0).

-- task018: Возвраты Yandex Market (раз в день, watermark + догон)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567818',
    'task018-ym-returns',
    'YM Возвраты — Partner API (раз в день, watermark + догон). Замените connection_id на UUID YM-кабинета.',
    'task018_ym_returns',
    '0 0 7 * * *',
    '{"connection_id":"REPLACE_WITH_YM_CONNECTION_ID","work_start_date":"2026-01-01","overlap_days":2,"chunk_days":14}',
    0, datetime('now'), datetime('now'), 0
);

-- task019: Отчёт по платежам Yandex Market (ночью — тяжёлый асинхронный отчёт)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567819',
    'task019-ym-payment-report',
    'YM Отчёт по платежам — Partner API united-netting (ночью, watermark + догон). Замените connection_id на UUID YM-кабинета.',
    'task019_ym_payment_report',
    '0 0 2 * * *',
    '{"connection_id":"REPLACE_WITH_YM_CONNECTION_ID","work_start_date":"2026-01-01","overlap_days":3,"chunk_days":14}',
    0, datetime('now'), datetime('now'), 0
);
