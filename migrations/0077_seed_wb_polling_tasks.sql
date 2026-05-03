-- Seed: два готовых задания для загрузки заказов WB
-- ВНИМАНИЕ: Перед включением заданий замените connection_id в config_json
--           на реальный UUID WB-подключения из справочника a006_connection_mp.
--           Задания создаются отключёнными (is_enabled = 0).

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
    'a1b2c3d4-e5f6-7890-abcd-ef1234567801',
    'task001-wb-orders-fbs',
    'WB Заказы FBS — адаптивный поллер (каждые 5 мин). Замените connection_id на UUID WB-кабинета.',
    'task001_wb_orders_fbs_polling',
    '0 */5 * * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","mode_threshold_minutes":6,"fallback_lookback_hours":24,"overlap_minutes":30}',
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
    'a1b2c3d4-e5f6-7890-abcd-ef1234567802',
    'task002-wb-orders-stats',
    'WB Заказы — полная история Statistics API (каждый час). Замените connection_id на UUID WB-кабинета.',
    'task002_wb_orders_stats_hourly',
    '0 0 * * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","lookback_days":7,"overlap_days":1}',
    0,
    datetime('now'),
    datetime('now'),
    0
);
