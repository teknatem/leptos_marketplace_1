-- Audit trail for manually activated LLM skill catalog snapshots.

CREATE TABLE IF NOT EXISTS sys_llm_skill_reload_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_user_id TEXT,
    previous_digest TEXT NOT NULL,
    new_digest TEXT NOT NULL,
    generation INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('activated', 'rejected')),
    diff_json TEXT NOT NULL DEFAULT '{}',
    diagnostics_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sys_llm_skill_reload_log_created_at
    ON sys_llm_skill_reload_log(created_at DESC);
