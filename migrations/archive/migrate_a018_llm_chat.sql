-- Migration: Create a018_llm_chat and a018_llm_chat_message tables
-- Description: LLM Chat aggregate for managing chat sessions with LLM agents and their messages
-- Date: 2026-01-17

-- Таблица чатов
CREATE TABLE IF NOT EXISTS a018_llm_chat (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    comment TEXT,
    agent_id TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (agent_id) REFERENCES a017_llm_agent(id)
);

-- Таблица сообщений чата
CREATE TABLE IF NOT EXISTS a018_llm_chat_message (
    id TEXT PRIMARY KEY,
    chat_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tokens_used INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (chat_id) REFERENCES a018_llm_chat(id) ON DELETE CASCADE
);

-- Индексы для таблицы чатов
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_code ON a018_llm_chat(code);
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_agent_id ON a018_llm_chat(agent_id);
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_is_deleted ON a018_llm_chat(is_deleted);
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_created_at ON a018_llm_chat(created_at);

-- Индексы для таблицы сообщений
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_chat_id ON a018_llm_chat_message(chat_id);
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_created_at ON a018_llm_chat_message(created_at);
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_role ON a018_llm_chat_message(role);
