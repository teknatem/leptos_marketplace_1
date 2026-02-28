-- Агрегат a022: Варианты комплектации номенклатуры (из УТ 11)
CREATE TABLE IF NOT EXISTS a022_kit_variant (
    id            TEXT PRIMARY KEY,
    code          TEXT NOT NULL DEFAULT '',
    description   TEXT NOT NULL DEFAULT '',
    comment       TEXT,
    owner_ref     TEXT,
    goods_json    TEXT,
    connection_id TEXT NOT NULL DEFAULT '',
    fetched_at    TEXT NOT NULL DEFAULT '',
    is_deleted    INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT,
    updated_at    TEXT,
    version       INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_a022_owner_ref    ON a022_kit_variant(owner_ref);
CREATE INDEX IF NOT EXISTS idx_a022_connection   ON a022_kit_variant(connection_id);
CREATE INDEX IF NOT EXISTS idx_a022_is_deleted   ON a022_kit_variant(is_deleted);
