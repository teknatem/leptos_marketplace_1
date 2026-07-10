-- User rating for an LLM chat (1..5; NULL = not rated).
ALTER TABLE a018_llm_chat ADD COLUMN rating INTEGER;
