SELECT
    MAX(p.loaded_at_utc) AS loaded_at_utc,
    MAX(SUBSTR(p.transaction_date, 1, 10)) AS last_transaction_date,
    MAX(SUBSTR(p.bank_order_date, 1, 10)) AS last_bank_order_date
FROM p907_ym_payment_report p
LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
WHERE (
    ? = ''
    OR COALESCE(
        CAST(p.business_id AS TEXT),
        NULLIF(c.business_account_id, ''),
        p.connection_mp_ref
    ) = ?
)
