CREATE TABLE sys_general_ledger_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    entry_date TEXT NOT NULL,
    layer TEXT NOT NULL,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    debit_account TEXT NOT NULL,
    credit_account TEXT NOT NULL,
    amount REAL NOT NULL,
    qty REAL,
    turnover_code TEXT NOT NULL,
    resource_table TEXT NOT NULL,
    resource_field TEXT NOT NULL DEFAULT 'amount',
    resource_sign INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

INSERT INTO sys_general_ledger_v2 (
    id,
    entry_date,
    layer,
    registrator_type,
    registrator_ref,
    debit_account,
    credit_account,
    amount,
    qty,
    turnover_code,
    resource_table,
    resource_field,
    resource_sign,
    created_at
)
SELECT
    id,
    entry_date,
    layer,
    registrator_type,
    registrator_ref,
    debit_account,
    credit_account,
    amount,
    qty,
    turnover_code,
    COALESCE(NULLIF(detail_kind, ''), NULLIF(registrator_type, ''), 'unknown'),
    COALESCE(NULLIF(resource_name, ''), 'amount'),
    COALESCE(resource_sign, 1),
    created_at
FROM sys_general_ledger;

DROP TABLE sys_general_ledger;

ALTER TABLE sys_general_ledger_v2 RENAME TO sys_general_ledger;

CREATE INDEX IF NOT EXISTS idx_sgl_registrator_ref
    ON sys_general_ledger (registrator_ref);

CREATE INDEX IF NOT EXISTS idx_sgl_turnover_code
    ON sys_general_ledger (turnover_code);

CREATE INDEX IF NOT EXISTS idx_sgl_entry_date
    ON sys_general_ledger (entry_date);
