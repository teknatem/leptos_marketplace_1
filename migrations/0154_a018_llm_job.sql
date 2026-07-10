CREATE TABLE IF NOT EXISTS a018_llm_job (
    id TEXT PRIMARY KEY,
    chat_id TEXT NOT NULL,
    request_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    progress_step INTEGER NOT NULL DEFAULT 0,
    progress_stage TEXT NOT NULL DEFAULT '',
    result_json TEXT,
    error TEXT,
    cancel_requested INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    FOREIGN KEY (chat_id) REFERENCES a018_llm_chat(id) ON DELETE CASCADE,
    UNIQUE (chat_id, request_id)
);

CREATE INDEX IF NOT EXISTS idx_a018_llm_job_expiry
    ON a018_llm_job(expires_at, status);
