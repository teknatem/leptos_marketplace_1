-- Image attachments for LLM chat.
-- chat_id makes pending uploads addressable and authorizable before message binding.
ALTER TABLE a018_llm_chat_attachment ADD COLUMN chat_id TEXT;

UPDATE a018_llm_chat_attachment
SET chat_id = (
    SELECT m.chat_id
    FROM a018_llm_chat_message m
    WHERE m.id = a018_llm_chat_attachment.message_id
)
WHERE chat_id IS NULL
  AND message_id <> '00000000-0000-0000-0000-000000000000';

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_attachment_chat_id
    ON a018_llm_chat_attachment(chat_id);

-- Curated subset of allowed_models that accepts image input.
ALTER TABLE a038_llm_connection ADD COLUMN image_input_models TEXT;
