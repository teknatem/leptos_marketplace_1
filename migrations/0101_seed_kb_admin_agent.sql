-- Seed a dedicated Knowledge Base administrator agent.
-- It inherits provider/model/key settings from the current primary LLM agent when present.

INSERT OR IGNORE INTO a017_llm_agent (
    id,
    code,
    description,
    comment,
    provider_type,
    api_endpoint,
    api_key,
    model_name,
    temperature,
    max_tokens,
    system_prompt,
    is_primary,
    is_deleted,
    is_posted,
    created_at,
    updated_at,
    version,
    available_models,
    agent_type
)
SELECT
    'a1b2c3d4-e5f6-7890-abcd-ef1234567818',
    'kb-admin',
    'Администратор базы знаний',
    'Создан автоматически для задач task014_kb_analyze и task015_kb_post.',
    provider_type,
    api_endpoint,
    api_key,
    model_name,
    0.2,
    max_tokens,
    NULL,
    0,
    0,
    0,
    datetime('now'),
    datetime('now'),
    1,
    available_models,
    'kb_admin'
FROM a017_llm_agent
WHERE is_deleted = 0
ORDER BY is_primary DESC, description ASC
LIMIT 1;
