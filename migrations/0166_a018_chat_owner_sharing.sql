-- Владелец чата (NULL = легаси-чаты до этой доработки) и признак общего доступа.
ALTER TABLE a018_llm_chat ADD COLUMN owner_user_id TEXT;
ALTER TABLE a018_llm_chat ADD COLUMN is_shared INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_owner ON a018_llm_chat(owner_user_id);
