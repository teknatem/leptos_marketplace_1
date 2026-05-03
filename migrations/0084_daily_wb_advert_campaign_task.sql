-- Ensure WB advert campaign catalog sync exists as a daily scheduled task.
-- 0083 initially seeded task012 more often; this migration normalizes the seed
-- without relying on editing an already published migration file.

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
)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567812',
    'task012-wb-advert-campaigns',
    'WB Реклама — справочник кампаний (ежедневно). Замените connection_id на UUID WB-кабинета.',
    'task012_wb_advert_campaigns',
    '0 30 5 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID"}',
    0,
    datetime('now'),
    datetime('now'),
    0
);

UPDATE sys_tasks
SET
    description = 'WB Реклама — справочник кампаний (ежедневно). Замените connection_id на UUID WB-кабинета.',
    schedule_cron = '0 30 5 * * *',
    updated_at = datetime('now')
WHERE code = 'task012-wb-advert-campaigns'
  AND task_type = 'task012_wb_advert_campaigns'
  AND is_deleted = 0;
