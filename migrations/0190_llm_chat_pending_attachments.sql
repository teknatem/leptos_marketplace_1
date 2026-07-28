-- A pasted image exists before its user message is sent, so message_id must be nullable.
-- SQLite requires rebuilding the table to remove the NOT NULL constraint.
CREATE TABLE a018_llm_chat_attachment_new (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT,
    message_id TEXT,
    filename TEXT NOT NULL,
    filepath TEXT NOT NULL,
    s3_file_id TEXT,
    content_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (chat_id) REFERENCES a018_llm_chat(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES a018_llm_chat_message(id) ON DELETE CASCADE
);

INSERT INTO a018_llm_chat_attachment_new (
    id, chat_id, message_id, filename, filepath, s3_file_id,
    content_type, file_size, created_at
)
SELECT
    id,
    chat_id,
    NULLIF(message_id, '00000000-0000-0000-0000-000000000000'),
    filename,
    filepath,
    s3_file_id,
    content_type,
    file_size,
    created_at
FROM a018_llm_chat_attachment;

DROP TABLE a018_llm_chat_attachment;
ALTER TABLE a018_llm_chat_attachment_new RENAME TO a018_llm_chat_attachment;

CREATE INDEX idx_a018_llm_chat_attachment_message_id
    ON a018_llm_chat_attachment(message_id);
CREATE INDEX idx_a018_llm_chat_attachment_chat_id
    ON a018_llm_chat_attachment(chat_id);
CREATE INDEX idx_a018_llm_chat_attachment_s3_file_id
    ON a018_llm_chat_attachment(s3_file_id);
