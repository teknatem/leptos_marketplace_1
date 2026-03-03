-- BI Dashboard aggregate (a025)
-- Дашборды BI. Один агрегат = один дашборд, содержащий коллекцию индикаторов.

CREATE TABLE IF NOT EXISTS a025_bi_dashboard (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,

    -- JSON-части дашборда
    layout_json TEXT NOT NULL DEFAULT '{"groups":[]}',
    global_filters_json TEXT NOT NULL DEFAULT '[]',

    -- Управление
    status TEXT NOT NULL DEFAULT 'draft',
    owner_user_id TEXT NOT NULL DEFAULT '',
    is_public INTEGER NOT NULL DEFAULT 0,
    rating INTEGER,
    created_by TEXT,
    updated_by TEXT,

    -- Стандартные поля BaseAggregate
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a025_bi_dashboard_status
    ON a025_bi_dashboard (status)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a025_bi_dashboard_owner
    ON a025_bi_dashboard (owner_user_id)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a025_bi_dashboard_public
    ON a025_bi_dashboard (is_public)
    WHERE is_deleted = 0;
