ALTER TABLE p903_wb_finance_report
    ADD COLUMN source_row_ref TEXT;

UPDATE p903_wb_finance_report
SET source_row_ref = 'p903:' || rr_dt || ':' || CAST(rrd_id AS TEXT)
WHERE source_row_ref IS NULL OR source_row_ref = '';

CREATE UNIQUE INDEX IF NOT EXISTS idx_p903_source_row_ref
    ON p903_wb_finance_report (source_row_ref);

ALTER TABLE sys_general_ledger
    ADD COLUMN resource_name TEXT;

ALTER TABLE sys_general_ledger
    ADD COLUMN resource_sign INTEGER;

UPDATE sys_general_ledger
SET resource_name = COALESCE(NULLIF(turnover_code, ''), 'unknown'),
    resource_sign = COALESCE(resource_sign, 1)
WHERE resource_name IS NULL OR resource_name = '' OR resource_sign IS NULL;

CREATE INDEX IF NOT EXISTS idx_sgl_detail_kind_detail_id
    ON sys_general_ledger (detail_kind, detail_id);

CREATE INDEX IF NOT EXISTS idx_sgl_registrator_ref
    ON sys_general_ledger (registrator_ref);
