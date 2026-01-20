-- Migration: Add artifact link to a018_llm_chat_message table
-- Description: Add artifact_id and artifact_action fields to support linking messages with artifacts
-- Date: 2026-01-20

-- Добавить artifact_id в таблицу сообщений
ALTER TABLE a018_llm_chat_message ADD COLUMN artifact_id TEXT;

-- Добавить artifact_action в таблицу сообщений
ALTER TABLE a018_llm_chat_message ADD COLUMN artifact_action TEXT;

-- Индекс для быстрого поиска сообщений по артефакту
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_artifact_id ON a018_llm_chat_message(artifact_id);
