CREATE TABLE IF NOT EXISTS plugin_snapshot (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    plugin_version INTEGER NOT NULL,
    method TEXT NOT NULL DEFAULT 'data',
    payload_json TEXT NOT NULL,
    row_count INTEGER NOT NULL DEFAULT 0,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    source_hash TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (plugin_id) REFERENCES plugin(id) ON DELETE CASCADE,
    UNIQUE (plugin_id, plugin_version, method)
);

CREATE INDEX IF NOT EXISTS idx_plugin_snapshot_current
    ON plugin_snapshot(plugin_id, plugin_version, method);

ALTER TABLE plugin_run ADD COLUMN data_mode TEXT NOT NULL DEFAULT 'live';

CREATE UNIQUE INDEX IF NOT EXISTS ux_plugin_revision_version
    ON plugin_revision(plugin_id, version);

ALTER TABLE plugin_revision ADD COLUMN source_spec_json TEXT;
ALTER TABLE plugin_revision ADD COLUMN source_hash TEXT;
ALTER TABLE plugin_revision ADD COLUMN snapshot_meta_json TEXT;
ALTER TABLE plugin_revision ADD COLUMN origin_json TEXT;
