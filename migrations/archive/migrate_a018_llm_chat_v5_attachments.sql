-- Migration: Add attachments support to a018_llm_chat
-- Description: Create table for storing file attachments linked to chat messages
-- Date: 2026-02-06

-- Создать таблицу для вложений
CREATE TABLE IF NOT EXISTS a018_llm_chat_attachment (
    id TEXT PRIMARY KEY NOT NULL,
    message_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    filepath TEXT NOT NULL,
    content_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES a018_llm_chat_message(id) ON DELETE CASCADE
);

-- Индекс для быстрого поиска вложений по сообщению
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_attachment_message_id 
ON a018_llm_chat_attachment(message_id);
