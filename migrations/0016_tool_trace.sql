-- Add tool call trace to LLM chat messages
ALTER TABLE a018_llm_chat_message ADD COLUMN tool_trace_json TEXT;
