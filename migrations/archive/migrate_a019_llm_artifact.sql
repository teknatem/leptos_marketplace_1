-- Migration: Create a019_llm_artifact table
-- Description: LLM Artifact aggregate for storing SQL queries and other artifacts created by LLM agents
-- Date: 2026-01-20

-- Таблица артефактов LLM
CREATE TABLE IF NOT EXISTS a019_llm_artifact (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    comment TEXT,
    
    -- Связи
    chat_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    
    -- Метаданные
    artifact_type TEXT NOT NULL DEFAULT 'sql_query',
    status TEXT NOT NULL DEFAULT 'active',
    
    -- SQL контент
    sql_query TEXT NOT NULL,
    query_params TEXT,
    visualization_config TEXT,
    
    -- Статистика выполнения
    last_executed_at TEXT,
    execution_count INTEGER NOT NULL DEFAULT 0,
    
    -- Стандартные поля
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    
    FOREIGN KEY (chat_id) REFERENCES a018_llm_chat(id),
    FOREIGN KEY (agent_id) REFERENCES a017_llm_agent(id)
);

-- Индексы для таблицы артефактов
CREATE INDEX IF NOT EXISTS idx_a019_artifact_code ON a019_llm_artifact(code);
CREATE INDEX IF NOT EXISTS idx_a019_artifact_chat_id ON a019_llm_artifact(chat_id);
CREATE INDEX IF NOT EXISTS idx_a019_artifact_agent_id ON a019_llm_artifact(agent_id);
CREATE INDEX IF NOT EXISTS idx_a019_artifact_type ON a019_llm_artifact(artifact_type);
CREATE INDEX IF NOT EXISTS idx_a019_artifact_status ON a019_llm_artifact(status);
CREATE INDEX IF NOT EXISTS idx_a019_artifact_is_deleted ON a019_llm_artifact(is_deleted);
CREATE INDEX IF NOT EXISTS idx_a019_artifact_created_at ON a019_llm_artifact(created_at);
