-- 0171: компакция истории чата a018.
-- summary_text — персистентная LLM-сводка ранней части диалога (вместо жёсткой
-- обрезки истории по количеству сообщений).
-- summary_upto — created_at (RFC3339) последнего сообщения, вошедшего в сводку:
-- сообщения старше этой метки в контекст не включаются (их заменяет сводка).
ALTER TABLE a018_llm_chat ADD COLUMN summary_text TEXT;
ALTER TABLE a018_llm_chat ADD COLUMN summary_upto TEXT;
