SELECT
    ROUND(COALESCE(SUM(
        CASE
            WHEN p.transaction_sum > 0 THEN p.transaction_sum
            ELSE 0
        END
    ), 0), 2) AS common_accruals,
    ROUND(COALESCE(SUM(
        CASE
            WHEN p.transaction_sum < 0 THEN p.transaction_sum
            ELSE 0
        END
    ), 0), 2) AS common_withholdings,
    ROUND(ABS(COALESCE(SUM(
        CASE
            WHEN p.transaction_sum < 0 THEN p.transaction_sum
            ELSE 0
        END
    ), 0)), 2) AS total_cost,
    ROUND(ABS(COALESCE(SUM(
        CASE
            WHEN p.transaction_sum < 0
             AND p.bank_order_id IS NOT NULL
            THEN p.transaction_sum
            ELSE 0
        END
    ), 0)), 2) AS settled_cost,
    ROUND(ABS(COALESCE(SUM(
        CASE
            WHEN p.transaction_sum < 0
             AND p.bank_order_id IS NULL
            THEN p.transaction_sum
            ELSE 0
        END
    ), 0)), 2) AS pending_cost
FROM p907_ym_payment_report p
LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
WHERE p.order_id IS NULL
  AND SUBSTR(p.transaction_date, 1, 10) BETWEEN ? AND ?
  AND COALESCE(p.payment_status, '') NOT LIKE 'Справочно:%'
  AND (
      ? = ''
      OR COALESCE(
          CAST(p.business_id AS TEXT),
          NULLIF(c.business_account_id, ''),
          p.connection_mp_ref
      ) = ?
  )
