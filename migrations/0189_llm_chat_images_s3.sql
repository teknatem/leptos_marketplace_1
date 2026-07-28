-- New LLM chat images are stored in the configured private S3 bucket.
-- filepath remains for backward-compatible reads of existing local attachments.
ALTER TABLE a018_llm_chat_attachment ADD COLUMN s3_file_id TEXT;

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_attachment_s3_file_id
    ON a018_llm_chat_attachment(s3_file_id);
