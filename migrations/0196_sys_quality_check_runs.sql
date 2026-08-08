CREATE TABLE IF NOT EXISTS sys_quality_check_runs (
    id                  TEXT PRIMARY KEY,
    check_id            TEXT NOT NULL,
    definition_digest   TEXT NOT NULL,
    input_json          TEXT NOT NULL DEFAULT '{}',
    trigger             TEXT NOT NULL DEFAULT 'manual',
    status              TEXT NOT NULL DEFAULT 'running',
    started_at          TEXT NOT NULL,
    finished_at         TEXT,
    duration_ms         INTEGER,
    population_total    INTEGER,
    violations_total    INTEGER,
    details_json        TEXT,
    error               TEXT
);

CREATE INDEX IF NOT EXISTS idx_quality_runs_check_started
    ON sys_quality_check_runs(check_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_quality_runs_status
    ON sys_quality_check_runs(status, started_at DESC);
