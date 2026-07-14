-- Seed почтового конвейера: приём/обработка (task021) и отправка ответов (task022).
-- Оба по умолчанию ВЫКЛЮЧЕНЫ (is_enabled=0) — включить после настройки [mail] в config.toml.
-- Кадэнс — каждые 5 минут; task022 смещён на минуту, чтобы отвечать после подготовки.

INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, created_at, updated_at, is_deleted
) VALUES (
    'b021b021-a039-4021-b021-000000000021',
    'task021-mail-intake',
    'Почта — приём и обработка запросов пользователей: классификация, прогон агента, подготовка ответа.',
    'task021_mail_intake',
    '0 */5 * * * *',
    '{"max_emails":5,"sla_minutes":60}',
    0,
    datetime('now'),
    datetime('now'),
    0
);

INSERT OR IGNORE INTO sys_tasks (
    id, code, description, task_type, schedule_cron, config_json,
    is_enabled, created_at, updated_at, is_deleted
) VALUES (
    'b022b022-a039-4022-b022-000000000022',
    'task022-mail-reply',
    'Почта — отправка ответов: проверка статусов/сроков и отправка писем пользователям.',
    'task022_mail_reply',
    '0 1-59/5 * * * *',
    '{"max_replies":10}',
    0,
    datetime('now'),
    datetime('now'),
    0
);
