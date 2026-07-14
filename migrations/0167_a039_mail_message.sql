-- a039_mail_message: «Письмо (журнал)» — краткие записи входящих/исходящих писем
-- почтового конвейера (поллинг INBOX → прогон LLM-агента → ответ пользователю).
-- Полное тело письма и переписки живёт в связанном чате a018; здесь только сводка,
-- статус обработки и ссылки на чат/сообщение/артефакт.
CREATE TABLE IF NOT EXISTS a039_mail_message (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    comment TEXT,
    direction TEXT NOT NULL,                 -- inbound | outbound
    imap_uid INTEGER,                        -- IMAP UID (входящие) для дедупа
    message_id_hdr TEXT,                     -- RFC Message-ID (тред + идемпотентность)
    in_reply_to_ref TEXT,                    -- id входящего a039 (для outbound)
    from_addr TEXT NOT NULL DEFAULT '',
    to_addr TEXT NOT NULL DEFAULT '',
    subject TEXT NOT NULL DEFAULT '',
    body_excerpt TEXT NOT NULL DEFAULT '',
    user_ref TEXT,                           -- sys_users.id связанного пользователя
    intent TEXT,
    agent_type TEXT,
    chat_ref TEXT,                           -- a018 chat id
    message_ref TEXT,                        -- assistant message id
    artifact_ref TEXT,                       -- a019 artifact id
    status TEXT NOT NULL DEFAULT 'received',
    error TEXT,
    due_at TEXT,                             -- RFC3339 срок обработки/ответа (SLA)
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a039_mail_uid ON a039_mail_message(direction, imap_uid);
CREATE INDEX IF NOT EXISTS idx_a039_mail_status ON a039_mail_message(direction, status);
