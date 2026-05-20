-- Migration: 0108 — a033_wb_day_close
-- Документ «Закрытие дня WB-кабинета».
-- Строки, проблемы и итоги хранятся как JSON-blob.
-- Уникальный индекс гарантирует ровно одну активную запись на (connection_id, business_date).

CREATE TABLE IF NOT EXISTS a033_wb_day_close (
    id                    TEXT    PRIMARY KEY NOT NULL,
    code                  TEXT    NOT NULL DEFAULT '',
    description           TEXT    NOT NULL DEFAULT '',
    comment               TEXT,
    connection_id         TEXT    NOT NULL,
    business_date         TEXT    NOT NULL,
    is_archived           INTEGER NOT NULL DEFAULT 0,
    archived_at           TEXT,
    archived_reason       TEXT,
    replaces_id           TEXT,
    last_recalculated_at  TEXT,
    snapshot_hash         TEXT    NOT NULL DEFAULT '',
    lines_json            TEXT    NOT NULL DEFAULT '[]',
    problems_json         TEXT    NOT NULL DEFAULT '[]',
    totals_json           TEXT    NOT NULL DEFAULT '{}',
    is_deleted            INTEGER NOT NULL DEFAULT 0,
    is_posted             INTEGER NOT NULL DEFAULT 0,
    created_at            TEXT,
    updated_at            TEXT,
    version               INTEGER NOT NULL DEFAULT 0
);

-- Ровно одна активная (не архивная, не удалённая) запись на (connection_id, business_date).
CREATE UNIQUE INDEX IF NOT EXISTS idx_a033_active_per_day
    ON a033_wb_day_close(connection_id, business_date)
    WHERE is_archived = 0 AND is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a033_business_date ON a033_wb_day_close(business_date);
CREATE INDEX IF NOT EXISTS idx_a033_connection     ON a033_wb_day_close(connection_id);
