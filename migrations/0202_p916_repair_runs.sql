CREATE TABLE IF NOT EXISTS sys_p916_repair_run (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    requested_by_user_id TEXT,
    payload_hash TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'completed_with_limitations', 'failed')),
    phase TEXT NOT NULL,
    precheck_json TEXT,
    postcheck_json TEXT,
    session_ids_json TEXT NOT NULL DEFAULT '[]',
    errors_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    UNIQUE(chat_id, payload_hash)
);

CREATE INDEX IF NOT EXISTS idx_sys_p916_repair_run_chat_created
    ON sys_p916_repair_run(chat_id, created_at DESC);

