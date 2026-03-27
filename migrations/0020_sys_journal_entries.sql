ALTER TABLE p909_mp_order_line_turnovers
    ADD COLUMN journal_entry_id TEXT;

CREATE INDEX IF NOT EXISTS idx_p909_journal_entry_id
    ON p909_mp_order_line_turnovers (journal_entry_id);

CREATE TABLE IF NOT EXISTS sys_journal_entries (
    id TEXT PRIMARY KEY NOT NULL,
    posting_id TEXT NOT NULL,
    entry_date TEXT NOT NULL,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    debit_account TEXT NOT NULL,
    credit_account TEXT NOT NULL,
    amount REAL NOT NULL,
    qty REAL,
    turnover_code TEXT NOT NULL,
    detail_kind TEXT NOT NULL,
    detail_id TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sje_posting_id
    ON sys_journal_entries (posting_id);
CREATE INDEX IF NOT EXISTS idx_sje_registrator_ref
    ON sys_journal_entries (registrator_ref);
CREATE INDEX IF NOT EXISTS idx_sje_entry_date
    ON sys_journal_entries (entry_date);
CREATE INDEX IF NOT EXISTS idx_sje_debit_date
    ON sys_journal_entries (debit_account, entry_date);
CREATE INDEX IF NOT EXISTS idx_sje_credit_date
    ON sys_journal_entries (credit_account, entry_date);
