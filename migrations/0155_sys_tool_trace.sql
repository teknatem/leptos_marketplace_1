-- Per-call tool trace log for a018 LLM chat.
-- One row per tool invocation (previously only a denormalized JSON blob lived on
-- a018_llm_chat_message.tool_trace_json). This table keeps the full input/output
-- payload so the UI can show a detailed call card, and enables cross-chat analytics
-- (latency, error rate, tool usage). The message JSON now holds only a minimal
-- pill summary ([{tool, ok, ms}]).
CREATE TABLE IF NOT EXISTS sys_tool_trace (
    id          TEXT PRIMARY KEY,
    chat_id     TEXT NOT NULL,
    message_id  TEXT NOT NULL,
    iteration   INTEGER NOT NULL,
    call_index  INTEGER NOT NULL,
    stage       TEXT NOT NULL,
    tool        TEXT NOT NULL,
    ok          INTEGER NOT NULL,
    ms          INTEGER NOT NULL,
    summary     TEXT,
    input_json  TEXT,
    output_json TEXT,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sys_tool_trace_message ON sys_tool_trace(message_id);
CREATE INDEX IF NOT EXISTS idx_sys_tool_trace_chat ON sys_tool_trace(chat_id);
CREATE INDEX IF NOT EXISTS idx_sys_tool_trace_tool ON sys_tool_trace(tool);
