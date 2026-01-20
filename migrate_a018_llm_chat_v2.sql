-- Migration: Add model_name and confidence to a018_llm_chat tables
-- Description: Add model selection and confidence tracking
-- Date: 2026-01-17

-- Добавить model_name в таблицу чатов
ALTER TABLE a018_llm_chat ADD COLUMN model_name TEXT NOT NULL DEFAULT 'gpt-4o';

-- Добавить model_name и confidence в таблицу сообщений
ALTER TABLE a018_llm_chat_message ADD COLUMN model_name TEXT;
ALTER TABLE a018_llm_chat_message ADD COLUMN confidence REAL;

-- Индекс для быстрого поиска по модели
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_model_name ON a018_llm_chat(model_name);
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_model_name ON a018_llm_chat_message(model_name);
