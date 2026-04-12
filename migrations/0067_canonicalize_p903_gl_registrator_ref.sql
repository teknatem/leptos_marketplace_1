-- Canonicalize historical p903 GL rows to use p903_wb_finance_report.id
-- instead of legacy registrator_ref aliases.

UPDATE sys_general_ledger
SET registrator_ref = (
    SELECT p903.id
    FROM p903_wb_finance_report p903
    WHERE p903.source_row_ref = sys_general_ledger.registrator_ref
    LIMIT 1
)
WHERE registrator_type = 'p903_wb_finance_report'
  AND registrator_ref LIKE 'p903:%'
  AND registrator_ref NOT LIKE 'p903:%:%'
  AND EXISTS (
      SELECT 1
      FROM p903_wb_finance_report p903
      WHERE p903.source_row_ref = sys_general_ledger.registrator_ref
  );

UPDATE sys_general_ledger
SET registrator_ref = (
    SELECT p903.id
    FROM p903_wb_finance_report p903
    WHERE p903.rr_dt = SUBSTR(sys_general_ledger.registrator_ref, 6, 10)
      AND p903.rrd_id = CAST(SUBSTR(sys_general_ledger.registrator_ref, 17) AS INTEGER)
    LIMIT 1
)
WHERE registrator_type = 'p903_wb_finance_report'
  AND registrator_ref LIKE 'p903:%:%'
  AND EXISTS (
      SELECT 1
      FROM p903_wb_finance_report p903
      WHERE p903.rr_dt = SUBSTR(sys_general_ledger.registrator_ref, 6, 10)
        AND p903.rrd_id = CAST(SUBSTR(sys_general_ledger.registrator_ref, 17) AS INTEGER)
  );
