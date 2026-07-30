-- Bitrix24 synchronization timestamps and comment metadata.
-- No separate synchronization log/state table is needed: scheduled task runs
-- are recorded by the existing sys_task_runs infrastructure.

ALTER TABLE sys_ticket ADD COLUMN bitrix_received_at TEXT;

ALTER TABLE sys_ticket_comment ADD COLUMN external_author_name TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_sys_ticket_comment_bitrix_id
    ON sys_ticket_comment (bitrix_comment_id)
    WHERE bitrix_comment_id IS NOT NULL;

INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, created_at, updated_at, is_deleted
) VALUES (
    'f9ab6a53-f154-4ee1-b7ed-46a16e7da025',
    'TASK025',
    'Синхронизация тикетов с задачами Битрикс24',
    'task025_bitrix_ticket_sync',
    '0 */5 * * * *',
    '{}',
    1,
    datetime('now'),
    datetime('now'),
    0
);
