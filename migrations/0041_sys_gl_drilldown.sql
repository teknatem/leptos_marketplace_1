CREATE TABLE IF NOT EXISTS sys_gl_drilldown (
    id           TEXT PRIMARY KEY NOT NULL,
    title        TEXT NOT NULL DEFAULT '',
    params_json  TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    last_used_at TEXT,
    use_count    INTEGER NOT NULL DEFAULT 0
);
