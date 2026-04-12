-- Speed up dv005_gl_account_view_total account balance queries.
-- These queries filter by:
--   (debit_account = ? OR credit_account = ?)
--   AND entry_date BETWEEN ? AND ?
-- Separate account/date indexes let SQLite use MULTI-INDEX OR instead of scanning by date only.

CREATE INDEX IF NOT EXISTS idx_sgl_debit_account_entry_date
ON sys_general_ledger(debit_account, entry_date);

CREATE INDEX IF NOT EXISTS idx_sgl_credit_account_entry_date
ON sys_general_ledger(credit_account, entry_date);
