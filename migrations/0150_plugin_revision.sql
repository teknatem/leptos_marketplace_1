-- Versioned plugin bundle snapshots for rollback/diff workflows.

CREATE TABLE IF NOT EXISTS plugin_revision (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    bundle_json TEXT NOT NULL DEFAULT '{}',
    validate_report_json TEXT NOT NULL DEFAULT '{}',
    smoke_report_json TEXT,
    created_by_agent_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_plugin_revision_plugin
    ON plugin_revision (plugin_id, version);

