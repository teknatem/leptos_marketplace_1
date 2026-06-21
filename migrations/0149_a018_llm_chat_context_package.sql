-- Page-context packages for LLM chats: a JSON snapshot of the current page
-- (identity + object data + adjacent refs), stored by UUID for reuse/memory and
-- injected into the chat conversation. Modelled after sys_drilldown.
CREATE TABLE IF NOT EXISTS a018_llm_chat_context_package (
    id            TEXT PRIMARY KEY NOT NULL,
    chat_id       TEXT,
    page_key      TEXT NOT NULL,
    page_type     TEXT NOT NULL,
    entity_index  TEXT,
    entity_id     TEXT,
    title         TEXT NOT NULL,
    context_json  TEXT NOT NULL,
    rendered_text TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    use_count     INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_a018_ctx_chat ON a018_llm_chat_context_package(chat_id);
