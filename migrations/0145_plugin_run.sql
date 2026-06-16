-- Журнал запусков плагинов (наблюдаемость подсистемы Plugins).
-- Пишется из service::invoke на каждый серверный вызов: длительность, статус и
-- стадия ошибки (PluginError.stage). Основа статистики и отклонений в UI.

CREATE TABLE IF NOT EXISTS plugin_run (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    method TEXT NOT NULL DEFAULT '',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    duration_ms INTEGER NOT NULL DEFAULT 0,
    -- ok | error | timeout
    status TEXT NOT NULL DEFAULT 'ok',
    -- стадия из PluginError (module_eval | missing_export | invoke | runtime | sql | deserialize | timeout)
    error_stage TEXT,
    row_count INTEGER,
    triggered_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_plugin_run_plugin
    ON plugin_run (plugin_id, started_at);

CREATE INDEX IF NOT EXISTS idx_plugin_run_started
    ON plugin_run (started_at);
