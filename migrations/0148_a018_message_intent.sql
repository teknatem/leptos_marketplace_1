-- Add router-detected intent label to LLM chat messages (Phase 0)
ALTER TABLE a018_llm_chat_message ADD COLUMN intent TEXT;
