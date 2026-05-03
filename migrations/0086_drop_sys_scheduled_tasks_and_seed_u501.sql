-- Drop legacy zombie table (pre-sqlx); active definitions live in sys_tasks.
-- Seed u501_import_ut if missing (was only present in sys_scheduled_tasks on some DBs).

INSERT INTO sys_tasks (id, code, description, task_type, is_enabled, is_deleted, created_at, updated_at)
SELECT
    lower(hex(randomblob(16))),
    'u501-import-ut',
    'Импорт из 1С:УТ',
    'u501_import_ut',
    0,
    0,
    datetime('now'),
    datetime('now')
WHERE NOT EXISTS (
    SELECT 1 FROM sys_tasks WHERE task_type = 'u501_import_ut' AND is_deleted = 0
);

DROP TABLE IF EXISTS sys_scheduled_tasks;
