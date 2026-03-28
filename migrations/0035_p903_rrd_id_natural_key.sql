CREATE UNIQUE INDEX IF NOT EXISTS idx_p903_rrd_id
    ON p903_wb_finance_report (rrd_id);

UPDATE p903_wb_finance_report
SET source_row_ref = 'p903:' || CAST(rrd_id AS TEXT)
WHERE source_row_ref IS NULL
   OR source_row_ref = ''
   OR source_row_ref != 'p903:' || CAST(rrd_id AS TEXT);
