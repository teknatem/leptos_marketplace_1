-- Drilldown session store.
-- Each row represents a set of drilldown parameters created by the frontend.
-- The row's id is embedded in the tab key ("drilldown__{id}"), so the page
-- can be fully restored after a browser refresh or shared via URL.

CREATE TABLE IF NOT EXISTS sys_drilldown (
    id             TEXT PRIMARY KEY NOT NULL,
    view_id        TEXT NOT NULL,
    indicator_id   TEXT NOT NULL DEFAULT '',
    indicator_name TEXT NOT NULL DEFAULT '',
    params_json    TEXT NOT NULL,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    last_used_at   TEXT,
    use_count      INTEGER NOT NULL DEFAULT 0
);
