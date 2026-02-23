-- Migration for d401 WB Finance Dashboard
-- Creates table for storing dashboard configurations

CREATE TABLE IF NOT EXISTS sys_dashboard_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    data_source TEXT NOT NULL,
    config_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_dashboard_configs_data_source 
ON sys_dashboard_configs(data_source);

CREATE INDEX IF NOT EXISTS idx_dashboard_configs_updated_at 
ON sys_dashboard_configs(updated_at DESC);
