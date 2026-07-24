SELECT COALESCE(SUM(p.transaction_sum), 0) AS pending_cost
FROM p907_ym_payment_report p
LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
WHERE p.bank_order_id IS NULL
  AND SUBSTR(p.transaction_date, 1, 10) BETWEEN ? AND ?
  AND p.transaction_sum < 0
  AND p.payment_status LIKE 'Будет удержан%'
  AND COALESCE(p.payment_status, '') NOT LIKE 'Справочно:%'
  AND (
      ? = ''
      OR COALESCE(
          CAST(p.business_id AS TEXT),
          NULLIF(c.business_account_id, ''),
          p.connection_mp_ref
      ) = ?
  )
