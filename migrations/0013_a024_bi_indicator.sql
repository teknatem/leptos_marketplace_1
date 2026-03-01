-- BI Indicator aggregate (a024)
-- Индикаторы BI-дашбордов. Один агрегат = один индикатор.

CREATE TABLE IF NOT EXISTS a024_bi_indicator (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment TEXT,

    -- 4 JSON-части индикатора
    data_spec_json TEXT NOT NULL DEFAULT '{}',
    params_json TEXT NOT NULL DEFAULT '[]',
    view_spec_json TEXT NOT NULL DEFAULT '{}',
    drill_spec_json TEXT,

    -- Управление
    status TEXT NOT NULL DEFAULT 'draft',
    owner_user_id TEXT NOT NULL DEFAULT '',
    is_public INTEGER NOT NULL DEFAULT 0,
    created_by TEXT,
    updated_by TEXT,

    -- Стандартные поля BaseAggregate
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a024_bi_indicator_status
    ON a024_bi_indicator (status)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a024_bi_indicator_owner
    ON a024_bi_indicator (owner_user_id)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a024_bi_indicator_public
    ON a024_bi_indicator (is_public)
    WHERE is_deleted = 0;
