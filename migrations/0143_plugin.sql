-- Plugins subsystem — надстройка над платформой (интерпретируемые расширения).
-- НЕ агрегат a0XX, а отдельная ветка; идентификация по свободному code.
-- Бандл (manifest + params + data + scripts + view_spec + styles + assets)
-- хранится JSON-колонками, как у a024.

CREATE TABLE IF NOT EXISTS plugin (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    -- client | server | hybrid
    runtime TEXT NOT NULL DEFAULT 'client',
    -- draft | active | disabled
    status TEXT NOT NULL DEFAULT 'draft',

    -- Бандл (JSON-части)
    manifest_json TEXT NOT NULL DEFAULT '{}',
    params_json TEXT NOT NULL DEFAULT '[]',
    data_json TEXT NOT NULL DEFAULT '{}',
    client_script TEXT,
    server_script TEXT,
    view_spec_json TEXT NOT NULL DEFAULT '{}',
    styles TEXT,
    assets_json TEXT NOT NULL DEFAULT '{}',

    -- Локальное состояние / владение
    owner_user_id TEXT,
    created_by_agent_id TEXT,
    is_enabled INTEGER NOT NULL DEFAULT 1,

    -- Стандартные служебные поля
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_plugin_code
    ON plugin (code)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_plugin_enabled
    ON plugin (is_enabled)
    WHERE is_deleted = 0;
