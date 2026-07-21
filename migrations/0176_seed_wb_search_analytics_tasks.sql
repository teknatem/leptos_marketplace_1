-- Seed: поисковая аналитика WB (a040/task024) по обоим кабинетам, раз в день.
-- Время cron в UTC (МСК = UTC+3). Сдвиг от task020 (06:00/18:00) и task023 (06:30/18:30):
--   '0 0 4,16 * * *' → 07:00 и 19:00 МСК — чтобы не биться за лимит кабинета.
-- Forward-only: WB отдаёт аналитику только за недавний период; включаем сразу.
-- Первый прогон сдвигаем на +50 минут, чтобы не стартовать одновременно с task020/023.

INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, next_run_at, created_at, updated_at, is_deleted
) VALUES (
    'c0240024-0000-4024-b024-000000000024',
    'task024-wb-search-analytics_SANSTAR',
    'WB Поисковая аналитика — показы/позиции (WB - SANSTAR, 07:00 и 19:00 МСК). Требует «Джем».',
    'task024_wb_search_analytics_daily',
    '0 0 4,16 * * *',
    '{"connection_id":"1386a311-1e26-4676-b696-8d577a119eec","window_days":1}',
    1,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now', '+50 minutes'),
    datetime('now'),
    datetime('now'),
    0
);

INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, next_run_at, created_at, updated_at, is_deleted
) VALUES (
    'c0240024-0000-4024-b024-000000000025',
    'task024-wb-search-analytics_CTC',
    'WB Поисковая аналитика — показы/позиции (WB2 - CTC, 07:00 и 19:00 МСК). Требует «Джем».',
    'task024_wb_search_analytics_daily',
    '0 0 4,16 * * *',
    '{"connection_id":"42e29532-72b1-4f38-be6e-38c331c61fe6","window_days":1}',
    1,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now', '+50 minutes'),
    datetime('now'),
    datetime('now'),
    0
);
