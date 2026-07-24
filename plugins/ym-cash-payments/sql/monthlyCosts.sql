WITH settled AS (
    SELECT
        COALESCE(
            CAST(p.business_id AS TEXT),
            NULLIF(c.business_account_id, ''),
            p.connection_mp_ref
        ) AS business_id,
        MAX(COALESCE(NULLIF(c.description, ''), NULLIF(p.shop_name, ''), 'Кабинет')) AS cabinet_name,
        SUBSTR(p.transaction_date, 1, 10) AS accrual_date,
        SUBSTR(COALESCE(p.act_date, ''), 1, 10) AS act_date,
        COALESCE(CAST(p.act_id AS TEXT), '') AS act_id,
        SUBSTR(p.bank_order_date, 1, 10) AS payment_date,
        CAST(p.bank_order_id AS TEXT) AS bank_order_id,
        COALESCE(p.transaction_source, 'Без источника') AS transaction_source,
        COALESCE(p.offer_or_service_name, 'Общее удержание') AS service_name,
        COALESCE(p.payment_status, '') AS payment_status,
        'settled' AS settlement_state,
        SUM(p.transaction_sum) AS amount,
        COUNT(*) AS rows_count
    FROM p907_ym_payment_report p
    LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
    WHERE p.order_id IS NULL
      AND p.bank_order_id IS NOT NULL
      AND p.bank_order_date IS NOT NULL
      AND TRIM(p.bank_order_date) <> ''
      AND SUBSTR(p.transaction_date, 1, 10) BETWEEN ? AND ?
      AND p.transaction_sum <> 0
      AND COALESCE(p.payment_status, '') NOT LIKE 'Справочно:%'
      AND (
          ? = ''
          OR COALESCE(
              CAST(p.business_id AS TEXT),
              NULLIF(c.business_account_id, ''),
              p.connection_mp_ref
          ) = ?
      )
    GROUP BY
        business_id,
        SUBSTR(p.transaction_date, 1, 10),
        SUBSTR(COALESCE(p.act_date, ''), 1, 10),
        p.act_id,
        SUBSTR(p.bank_order_date, 1, 10),
        p.bank_order_id,
        p.transaction_source,
        p.offer_or_service_name,
        p.payment_status
),
pending AS (
    SELECT
        COALESCE(
            CAST(p.business_id AS TEXT),
            NULLIF(c.business_account_id, ''),
            p.connection_mp_ref
        ) AS business_id,
        MAX(COALESCE(NULLIF(c.description, ''), NULLIF(p.shop_name, ''), 'Кабинет')) AS cabinet_name,
        SUBSTR(p.transaction_date, 1, 10) AS accrual_date,
        SUBSTR(COALESCE(p.act_date, ''), 1, 10) AS act_date,
        COALESCE(CAST(p.act_id AS TEXT), '') AS act_id,
        '' AS payment_date,
        '' AS bank_order_id,
        COALESCE(p.transaction_source, 'Без источника') AS transaction_source,
        COALESCE(p.offer_or_service_name, 'Общее удержание') AS service_name,
        COALESCE(p.payment_status, '') AS payment_status,
        'pending' AS settlement_state,
        SUM(p.transaction_sum) AS amount,
        COUNT(*) AS rows_count
    FROM p907_ym_payment_report p
    LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
    WHERE p.order_id IS NULL
      AND p.bank_order_id IS NULL
      AND SUBSTR(p.transaction_date, 1, 10) BETWEEN ? AND ?
      AND p.transaction_sum <> 0
      AND COALESCE(p.payment_status, '') NOT LIKE 'Справочно:%'
      AND (
          ? = ''
          OR COALESCE(
              CAST(p.business_id AS TEXT),
              NULLIF(c.business_account_id, ''),
              p.connection_mp_ref
          ) = ?
      )
    GROUP BY
        business_id,
        SUBSTR(p.transaction_date, 1, 10),
        SUBSTR(COALESCE(p.act_date, ''), 1, 10),
        p.act_id,
        p.transaction_source,
        p.offer_or_service_name,
        p.payment_status
)
SELECT
    business_id,
    cabinet_name,
    accrual_date,
    act_date,
    act_id,
    payment_date,
    bank_order_id,
    transaction_source,
    service_name,
    payment_status,
    settlement_state,
    amount,
    rows_count
FROM (
    SELECT
        business_id,
        cabinet_name,
        accrual_date,
        act_date,
        act_id,
        payment_date,
        bank_order_id,
        transaction_source,
        service_name,
        payment_status,
        settlement_state,
        amount,
        rows_count
    FROM settled
    UNION ALL
    SELECT
        business_id,
        cabinet_name,
        accrual_date,
        act_date,
        act_id,
        payment_date,
        bank_order_id,
        transaction_source,
        service_name,
        payment_status,
        settlement_state,
        amount,
        rows_count
    FROM pending
)
ORDER BY accrual_date DESC, ABS(amount) DESC
