-- WB Returns Claims aggregate (a032)
-- Заявки покупателей на возврат товара Wildberries.
-- Источник: GET https://feedbacks-api.wildberries.ru/api/v1/claims
-- Уnikальный ключ: (claim_id, connection_id)

CREATE TABLE IF NOT EXISTS a032_wb_returns_claims (
    id          TEXT PRIMARY KEY NOT NULL,
    code        TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    comment     TEXT,

    connection_id   TEXT NOT NULL DEFAULT '',
    organization_id TEXT NOT NULL DEFAULT '',
    marketplace_id  TEXT NOT NULL DEFAULT '',

    claim_id     TEXT NOT NULL,
    claim_type   INTEGER,
    status       INTEGER,
    status_ex    INTEGER,
    nm_id        INTEGER NOT NULL DEFAULT 0,
    imt_name     TEXT,
    user_comment TEXT,
    wb_comment   TEXT,

    dt           TEXT NOT NULL,
    order_dt     TEXT,
    dt_update    TEXT,
    delivery_dt  TEXT,

    price         REAL,
    currency_code TEXT,
    srid          TEXT,
    origin_id_info TEXT,
    actions       TEXT,
    is_archive    INTEGER NOT NULL DEFAULT 0,

    is_deleted  INTEGER NOT NULL DEFAULT 0,
    is_posted   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT,
    updated_at  TEXT,
    version     INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a032_wb_returns_claims_claim_connection
    ON a032_wb_returns_claims(claim_id, connection_id)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a032_wb_returns_claims_connection_id
    ON a032_wb_returns_claims(connection_id);

CREATE INDEX IF NOT EXISTS idx_a032_wb_returns_claims_nm_id
    ON a032_wb_returns_claims(nm_id);

CREATE INDEX IF NOT EXISTS idx_a032_wb_returns_claims_dt
    ON a032_wb_returns_claims(dt);

CREATE INDEX IF NOT EXISTS idx_a032_wb_returns_claims_status
    ON a032_wb_returns_claims(status);

CREATE INDEX IF NOT EXISTS idx_a032_wb_returns_claims_is_archive
    ON a032_wb_returns_claims(is_archive);
