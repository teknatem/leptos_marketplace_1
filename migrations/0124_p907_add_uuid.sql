-- 0124 p907_ym_payment_report: add uuid id column
-- id is the stable internal reference used for UI navigation and internal links.
-- record_key is the business deduplication key (ymid_... format).
--
-- SQLite does not allow non-constant defaults in ALTER TABLE ADD COLUMN,
-- so we add the column with an empty default first, then fill existing rows,
-- then add the unique index.
ALTER TABLE p907_ym_payment_report
    ADD COLUMN id TEXT NOT NULL DEFAULT '';

UPDATE p907_ym_payment_report
    SET id = lower(hex(randomblob(16)))
    WHERE id = '';

CREATE UNIQUE INDEX IF NOT EXISTS idx_p907_id ON p907_ym_payment_report (id);
