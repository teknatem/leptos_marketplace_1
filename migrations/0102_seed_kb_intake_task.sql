-- Seed Knowledge Base business knowledge intake task.

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
    'a1b2c3d4-e5f6-7890-abcd-ef1234567819',
    'task016-kb-intake',
    'KB — сбор бизнес-знаний: создаёт небольшие вопросные тикеты для пополнения базы знаний о работе фирмы.',
    'task016_kb_intake',
    '0 0 11 * * *',
    '{"max_tickets":1,"questions_per_ticket":5}',
    1,
    datetime('now'),
    datetime('now'),
    0
);

UPDATE sys_tasks
SET
    description = 'KB — аудит качества ответов: проверяет историю чатов на плохие ответы и создаёт тикеты улучшения.',
    updated_at = datetime('now')
WHERE code = 'task014-kb-analyze';
