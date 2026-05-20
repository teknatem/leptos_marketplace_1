-- =============================================================================
-- Migration 0110: System favorites
-- =============================================================================

CREATE TABLE IF NOT EXISTS sys_favorites (
    id             TEXT PRIMARY KEY,
    owner_user_id  TEXT NOT NULL,
    target_kind    TEXT NOT NULL,
    target_id      TEXT NOT NULL,
    target_title   TEXT NOT NULL,
    tab_key        TEXT NOT NULL,
    color          TEXT NOT NULL DEFAULT 'yellow',
    comment        TEXT,
    is_global      INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_sys_favorites_owner_target
    ON sys_favorites(owner_user_id, target_kind, target_id);

CREATE INDEX IF NOT EXISTS ix_sys_favorites_owner
    ON sys_favorites(owner_user_id);

CREATE INDEX IF NOT EXISTS ix_sys_favorites_global
    ON sys_favorites(is_global);

CREATE INDEX IF NOT EXISTS ix_sys_favorites_target
    ON sys_favorites(target_kind, target_id);
