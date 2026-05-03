-- История запусков регламентных заданий
CREATE TABLE IF NOT EXISTS sys_task_runs (
    id              TEXT PRIMARY KEY,
    task_id         TEXT NOT NULL,
    session_id      TEXT NOT NULL UNIQUE,
    triggered_by    TEXT NOT NULL DEFAULT 'Scheduled',  -- 'Scheduled' | 'Manual'
    started_at      DATETIME NOT NULL,
    finished_at     DATETIME,
    duration_ms     INTEGER,
    status          TEXT NOT NULL DEFAULT 'Running',    -- 'Running' | 'Completed' | 'Failed'
    total_processed INTEGER,
    total_inserted  INTEGER,
    total_updated   INTEGER,
    total_errors    INTEGER,
    log_file_path   TEXT,
    error_message   TEXT,
    FOREIGN KEY (task_id) REFERENCES sys_tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_sys_task_runs_task_id    ON sys_task_runs(task_id);
CREATE INDEX IF NOT EXISTS idx_sys_task_runs_started_at ON sys_task_runs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sys_task_runs_status     ON sys_task_runs(status);
