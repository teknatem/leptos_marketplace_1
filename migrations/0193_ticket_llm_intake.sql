-- Оформление тикетов из AI-чата: обратная ссылка на диалог-исток и выдача
-- навыка `support` специализации разработчика.

ALTER TABLE sys_ticket ADD COLUMN source_chat_id TEXT;

CREATE INDEX IF NOT EXISTS idx_sys_ticket_source_chat
    ON sys_ticket(source_chat_id) WHERE source_chat_id IS NOT NULL;

-- Разработчик (код специализации остался `plugin_admin`) ведёт поддержку пользователей.
INSERT OR IGNORE INTO sys_llm_skill_access (specialization, skill_id, access_level) VALUES
    ('plugin_admin', 'support', 'immediate'),
    ('coordinator_admin', 'support', 'immediate');
