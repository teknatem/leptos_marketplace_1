-- Добавить поле agent_type в таблицу a017_llm_agent
-- Значения: 'business_analyst' | 'system_admin' | 'general'
ALTER TABLE a017_llm_agent ADD COLUMN agent_type TEXT NOT NULL DEFAULT 'business_analyst';

-- Все существующие агенты — аналитики
UPDATE a017_llm_agent SET agent_type = 'business_analyst';
