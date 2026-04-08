ALTER TABLE a027_wb_documents
    ADD COLUMN is_weekly_report INTEGER NOT NULL DEFAULT 0;

ALTER TABLE a027_wb_documents
    ADD COLUMN report_period_from TEXT;

ALTER TABLE a027_wb_documents
    ADD COLUMN report_period_to TEXT;

ALTER TABLE a027_wb_documents
    ADD COLUMN weekly_report_manual_json TEXT NOT NULL DEFAULT '{}';

UPDATE a027_wb_documents
SET is_weekly_report = 1
WHERE category = 'Еженедельный отчет реализации';

CREATE INDEX IF NOT EXISTS idx_a027_wb_documents_is_weekly_report
    ON a027_wb_documents(is_weekly_report);
