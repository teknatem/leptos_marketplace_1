-- a027 weekly report: persist maximum reconciliation deviation (by absolute amount)
-- computed at posting time, used for display in the document list ("Проверка" tab).

ALTER TABLE a027_wb_documents
    ADD COLUMN max_deviation REAL;
