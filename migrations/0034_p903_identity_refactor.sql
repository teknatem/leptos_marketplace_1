ALTER TABLE p903_wb_finance_report
    ADD COLUMN id TEXT;

UPDATE p903_wb_finance_report
SET id = lower(hex(randomblob(4))) || '-' ||
         lower(hex(randomblob(2))) || '-' ||
         lower(hex(randomblob(2))) || '-' ||
         lower(hex(randomblob(2))) || '-' ||
         lower(hex(randomblob(6)))
WHERE id IS NULL OR id = '';

CREATE UNIQUE INDEX IF NOT EXISTS idx_p903_id
    ON p903_wb_finance_report (id);
