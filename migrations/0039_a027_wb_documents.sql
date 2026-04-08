CREATE TABLE IF NOT EXISTS a027_wb_documents (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,
    service_name TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    creation_time TEXT NOT NULL DEFAULT '',
    viewed INTEGER NOT NULL DEFAULT 0,
    connection_id TEXT NOT NULL DEFAULT '',
    organization_id TEXT NOT NULL DEFAULT '',
    marketplace_id TEXT NOT NULL DEFAULT '',
    extensions_json TEXT NOT NULL DEFAULT '[]',
    source_meta_json TEXT NOT NULL DEFAULT '{}',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a027_wb_documents_connection_service_name
    ON a027_wb_documents(connection_id, service_name)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a027_wb_documents_creation_time
    ON a027_wb_documents(creation_time);

CREATE INDEX IF NOT EXISTS idx_a027_wb_documents_connection_id
    ON a027_wb_documents(connection_id);

CREATE INDEX IF NOT EXISTS idx_a027_wb_documents_service_name
    ON a027_wb_documents(service_name);

CREATE INDEX IF NOT EXISTS idx_a027_wb_documents_name
    ON a027_wb_documents(name);
