-- Seed: two disabled Yandex Market order polling tasks for separate cabinets.
-- Before enabling, replace connection_id placeholders with real a006_connection_mp UUIDs.

INSERT OR IGNORE INTO sys_tasks (
    id,
    code,
    description,
    task_type,
    schedule_cron,
    config_json,
    is_enabled,
    created_at,
    updated_at,
    is_deleted
) VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567813',
    'task013-ym-orders-polling-cabinet-1',
    'YM Заказы — адаптивный поллер для кабинета 1 (каждые 5 мин). Замените connection_id на UUID YM-кабинета.',
    'task013_ym_orders_polling',
    '0 */5 * * * *',
    '{"connection_id":"REPLACE_WITH_YM_CONNECTION_ID_1","mode_threshold_minutes":6,"fallback_lookback_hours":24,"overlap_minutes":30}',
    0,
    datetime('now'),
    datetime('now'),
    0
);

INSERT OR IGNORE INTO sys_tasks (
    id,
    code,
    description,
    task_type,
    schedule_cron,
    config_json,
    is_enabled,
    created_at,
    updated_at,
    is_deleted
) VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567814',
    'task013-ym-orders-polling-cabinet-2',
    'YM Заказы — адаптивный поллер для кабинета 2 (каждые 5 мин). Замените connection_id на UUID YM-кабинета.',
    'task013_ym_orders_polling',
    '0 */5 * * * *',
    '{"connection_id":"REPLACE_WITH_YM_CONNECTION_ID_2","mode_threshold_minutes":6,"fallback_lookback_hours":24,"overlap_minutes":30}',
    0,
    datetime('now'),
    datetime('now'),
    0
);
