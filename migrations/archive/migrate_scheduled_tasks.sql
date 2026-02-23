-- Scheduled Tasks system table
CREATE TABLE IF NOT EXISTS sys_tasks (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT,
    task_type TEXT NOT NULL,
    schedule_cron TEXT,
    config_json TEXT,
    is_enabled INTEGER DEFAULT 1,
    last_run_at TEXT,
    next_run_at TEXT,
    last_run_status TEXT,
    last_run_log_file TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    is_deleted INTEGER DEFAULT 0
);

