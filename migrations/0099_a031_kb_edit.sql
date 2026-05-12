-- Knowledge Base Edit aggregate (a031)
-- Тикеты "Редактирование базы знаний" для агента-администратора KB.

CREATE TABLE IF NOT EXISTS a031_kb_edit (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,

    edit_type TEXT NOT NULL DEFAULT 'proposal',
    status TEXT NOT NULL DEFAULT 'pending',
    title TEXT NOT NULL DEFAULT '',
    agent_summary TEXT NOT NULL DEFAULT '',

    target_articles TEXT NOT NULL DEFAULT '[]',
    applied_articles TEXT NOT NULL DEFAULT '[]',
    source_chat_ids TEXT NOT NULL DEFAULT '[]',

    agent_id TEXT,
    chat_id TEXT,
    analyze_task_run_id TEXT,
    post_task_run_id TEXT,

    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a031_kb_edit_status
    ON a031_kb_edit (status)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a031_kb_edit_edit_type
    ON a031_kb_edit (edit_type)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a031_kb_edit_chat
    ON a031_kb_edit (chat_id)
    WHERE is_deleted = 0;
