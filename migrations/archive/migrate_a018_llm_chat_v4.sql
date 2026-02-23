-- Migration: Add duration_ms to a018_llm_chat_message table
-- Description: Add duration_ms field to track LLM response execution time
-- Date: 2026-02-06

-- Добавить duration_ms в таблицу сообщений
ALTER TABLE a018_llm_chat_message ADD COLUMN duration_ms INTEGER;
