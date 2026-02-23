-- Migration: Create a017_llm_agent table
-- Description: LLM Agent aggregate for managing AI provider configurations
-- Date: 2026-01-17

CREATE TABLE IF NOT EXISTS a017_llm_agent (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    comment TEXT,
    provider_type TEXT NOT NULL DEFAULT 'OpenAI',
    api_endpoint TEXT NOT NULL,
    api_key TEXT NOT NULL,
    model_name TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INTEGER NOT NULL DEFAULT 4096,
    system_prompt TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_code ON a017_llm_agent(code);
CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_provider_type ON a017_llm_agent(provider_type);
CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_is_primary ON a017_llm_agent(is_primary);
CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_is_deleted ON a017_llm_agent(is_deleted);
