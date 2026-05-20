-- =============================================================================
-- Migration 0118: Page open history ("История открытых страниц")
-- =============================================================================
-- Per-user log of opened pages (tabs). Recorded centrally on every open_tab.
-- Used by the history drawer in the header to browse and re-open pages.

CREATE TABLE IF NOT EXISTS sys_page_history (
    id             TEXT PRIMARY KEY,
    owner_user_id  TEXT NOT NULL,
    tab_key        TEXT NOT NULL,
    title          TEXT NOT NULL,
    opened_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS ix_sys_page_history_owner_time
    ON sys_page_history(owner_user_id, opened_at DESC);
