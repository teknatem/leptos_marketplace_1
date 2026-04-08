CREATE TABLE sys_general_ledger_v3 (
    id TEXT PRIMARY KEY NOT NULL,
    entry_date TEXT NOT NULL,
    layer TEXT NOT NULL,
    connection_mp_ref TEXT,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    order_id TEXT,
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

INSERT INTO sys_general_ledger_v3 (
    id,
    entry_date,
    layer,
    connection_mp_ref,
    registrator_type,
    registrator_ref,
    order_id,
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
    gl.id,
    gl.entry_date,
    gl.layer,
    gl.cabinet_mp,
    gl.registrator_type,
    CASE
        WHEN gl.registrator_type = 'a012_wb_sales'
            AND gl.registrator_ref LIKE 'a012:%'
            THEN SUBSTR(gl.registrator_ref, 6)
        WHEN gl.registrator_type = 'a026_wb_advert_daily'
            AND gl.registrator_ref LIKE 'a026:%'
            THEN SUBSTR(gl.registrator_ref, 6)
        WHEN gl.registrator_type = 'p903_wb_finance_report'
            AND gl.registrator_ref LIKE 'p903:%'
            THEN COALESCE(
                (
                    SELECT p903.id
                    FROM p903_wb_finance_report p903
                    WHERE p903.source_row_ref = gl.registrator_ref
                    LIMIT 1
                ),
                gl.registrator_ref
            )
        ELSE gl.registrator_ref
    END,
    CASE
        WHEN gl.registrator_type = 'a012_wb_sales'
            THEN (
                SELECT a012.document_no
                FROM a012_wb_sales a012
                WHERE a012.id = CASE
                    WHEN gl.registrator_ref LIKE 'a012:%'
                        THEN SUBSTR(gl.registrator_ref, 6)
                    ELSE gl.registrator_ref
                END
                LIMIT 1
            )
        WHEN gl.registrator_type = 'p903_wb_finance_report'
            THEN (
                SELECT NULLIF(TRIM(p903.srid), '')
                FROM p903_wb_finance_report p903
                WHERE p903.id = CASE
                    WHEN gl.registrator_ref LIKE 'p903:%'
                        THEN COALESCE(
                            (
                                SELECT p903_by_ref.id
                                FROM p903_wb_finance_report p903_by_ref
                                WHERE p903_by_ref.source_row_ref = gl.registrator_ref
                                LIMIT 1
                            ),
                            gl.registrator_ref
                        )
                    ELSE gl.registrator_ref
                END
                LIMIT 1
            )
        ELSE NULL
    END,
    gl.debit_account,
    gl.credit_account,
    gl.amount,
    gl.qty,
    gl.turnover_code,
    gl.resource_table,
    gl.resource_field,
    gl.resource_sign,
    gl.created_at
FROM sys_general_ledger gl;

DROP TABLE sys_general_ledger;

ALTER TABLE sys_general_ledger_v3 RENAME TO sys_general_ledger;

DROP INDEX IF EXISTS idx_sgl_registrator_ref;
DROP INDEX IF EXISTS idx_sgl_turnover_code;
DROP INDEX IF EXISTS idx_sgl_entry_date;
DROP INDEX IF EXISTS idx_sgl_cabinet_mp;

CREATE INDEX IF NOT EXISTS idx_sgl_registrator_identity
    ON sys_general_ledger (registrator_type, registrator_ref);

CREATE INDEX IF NOT EXISTS idx_sgl_connection_mp_ref
    ON sys_general_ledger (connection_mp_ref);

CREATE INDEX IF NOT EXISTS idx_sgl_order_id
    ON sys_general_ledger (order_id);

CREATE INDEX IF NOT EXISTS idx_sgl_turnover_code
    ON sys_general_ledger (turnover_code);

CREATE INDEX IF NOT EXISTS idx_sgl_entry_date
    ON sys_general_ledger (entry_date);
