-- Тикеты: обратная связь пользователей (вопрос / доработка / ошибка / идея).
-- Структура полей частично унифицирована с задачей Битрикс24 (tasks.task.*):
-- title↔TITLE, description↔DESCRIPTION, priority↔PRIORITY, deadline↔DEADLINE,
-- author_user_id↔CREATED_BY, assignee_user_id↔RESPONSIBLE_ID, tags↔TAGS.
-- Поля bitrix_*, sync_status, origin, context_* — задел под интеграцию
-- с Битрикс24 и полуавтоматическое создание из LLM-чата.

CREATE TABLE IF NOT EXISTS sys_ticket (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    ticket_type TEXT NOT NULL DEFAULT 'question',
    status TEXT NOT NULL DEFAULT 'new',
    priority TEXT NOT NULL DEFAULT 'normal',
    deadline TEXT,
    author_user_id TEXT NOT NULL,
    assignee_user_id TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    origin TEXT NOT NULL DEFAULT 'self',
    context_page_key TEXT,
    context_json TEXT,
    bitrix_task_id TEXT,
    bitrix_synced_at TEXT,
    sync_status TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sys_ticket_code
    ON sys_ticket (code)
    WHERE code <> '';

CREATE INDEX IF NOT EXISTS idx_sys_ticket_author
    ON sys_ticket (author_user_id)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_sys_ticket_status
    ON sys_ticket (status)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_sys_ticket_updated_at
    ON sys_ticket (updated_at);

CREATE TABLE IF NOT EXISTS sys_ticket_comment (
    id TEXT PRIMARY KEY,
    ticket_id TEXT NOT NULL,
    author_user_id TEXT NOT NULL,
    body TEXT NOT NULL,
    origin TEXT NOT NULL DEFAULT 'self',
    bitrix_comment_id TEXT,
    created_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (ticket_id) REFERENCES sys_ticket(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sys_ticket_comment_ticket
    ON sys_ticket_comment (ticket_id)
    WHERE is_deleted = 0;

CREATE TABLE IF NOT EXISTS sys_ticket_attachment (
    id TEXT PRIMARY KEY,
    ticket_id TEXT NOT NULL,
    comment_id TEXT,
    s3_file_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    content_type TEXT,
    file_size INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (ticket_id) REFERENCES sys_ticket(id) ON DELETE CASCADE,
    FOREIGN KEY (comment_id) REFERENCES sys_ticket_comment(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sys_ticket_attachment_ticket
    ON sys_ticket_attachment (ticket_id);

CREATE INDEX IF NOT EXISTS idx_sys_ticket_attachment_comment
    ON sys_ticket_attachment (comment_id);

CREATE INDEX IF NOT EXISTS idx_sys_ticket_attachment_s3_file
    ON sys_ticket_attachment (s3_file_id);
