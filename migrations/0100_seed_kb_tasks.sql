-- Seed Knowledge Base administration scheduled tasks.
-- Both tasks can be run manually from the existing task UI.

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
    'a1b2c3d4-e5f6-7890-abcd-ef1234567816',
    'task014-kb-analyze',
    'KB — анализ базы знаний: создаёт тикеты a031_kb_edit с концепциями правок.',
    'task014_kb_analyze',
    '0 0 3 * * *',
    '{"lookback_days":7}',
    1,
    datetime('now'),
    datetime('now'),
    0
);

INSERT OR IGNORE INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, 'a031_kb_edit', 'all'
FROM sys_roles
WHERE code = 'manager';

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
    'a1b2c3d4-e5f6-7890-abcd-ef1234567817',
    'task015-kb-post',
    'KB — публикация правок: обрабатывает утверждённые тикеты и записывает статьи.',
    'task015_kb_post',
    '0 0 * * * *',
    '{}',
    1,
    datetime('now'),
    datetime('now'),
    0
);
