-- Global system settings key-value store.
-- Used to persist cross-cutting configuration such as the scheduler on/off switch.
CREATE TABLE IF NOT EXISTS sys_settings (
    key        TEXT NOT NULL PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed: scheduler enabled by default.
INSERT OR IGNORE INTO sys_settings (key, value) VALUES ('scheduler_enabled', 'true');
