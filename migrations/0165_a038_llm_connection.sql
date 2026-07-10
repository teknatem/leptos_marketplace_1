-- a038_llm_connection: «Подключение LLM» (провайдер + креды + модели + персона).
-- Новый агрегат-замена концепции a017 «Агент». Отличие от a017 — поле allowed_models
-- (курируемый короткий список разрешённых моделей). a017 остаётся нетронутым.
CREATE TABLE IF NOT EXISTS a038_llm_connection (
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
    available_models TEXT,
    allowed_models TEXT,
    agent_type TEXT NOT NULL DEFAULT 'business_analyst',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

-- Одноразовая копия существующих агентов a017 -> подключения a038 с сохранением тех же UUID.
-- Чат хранит связь как UUID-строку (chat.agent_id), поэтому одинаковые id делают существующие
-- чаты валидными без миграции таблицы чата.
-- allowed_models по умолчанию = JSON-массив из одной текущей модели (курировать вручную в UI).
INSERT OR IGNORE INTO a038_llm_connection (
    id, code, description, comment, provider_type, api_endpoint, api_key,
    model_name, temperature, max_tokens, system_prompt, is_primary,
    available_models, allowed_models, agent_type,
    is_deleted, is_posted, created_at, updated_at, version
)
SELECT
    id, code, description, comment, provider_type, api_endpoint, api_key,
    model_name, temperature, max_tokens, system_prompt, is_primary,
    available_models,
    json_array(model_name) AS allowed_models,
    agent_type,
    is_deleted, is_posted, created_at, updated_at, version
FROM a017_llm_agent
WHERE is_deleted = 0;
