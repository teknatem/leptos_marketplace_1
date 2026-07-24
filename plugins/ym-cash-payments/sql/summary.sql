WITH monetary AS (
    SELECT
        COALESCE(
            CAST(p.business_id AS TEXT),
            NULLIF(c.business_account_id, ''),
            p.connection_mp_ref
        ) AS business_id,
        p.bank_order_id,
        SUBSTR(p.bank_order_date, 1, 10) AS payment_date,
        p.order_id,
        COALESCE(p.transaction_sum, 0) AS transaction_sum,
        COALESCE(p.transaction_source, '') AS transaction_source,
        COALESCE(p.payment_status, '') AS payment_status,
        COALESCE(p.bank_sum, 0) AS bank_sum
    FROM p907_ym_payment_report p
    LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
    WHERE p.bank_order_id IS NOT NULL
      AND p.bank_order_date IS NOT NULL
      AND TRIM(p.bank_order_date) <> ''
      AND SUBSTR(p.bank_order_date, 1, 10) BETWEEN ? AND ?
      AND COALESCE(p.payment_status, '') NOT LIKE 'Справочно:%'
      AND (
          ? = ''
          OR COALESCE(
              CAST(p.business_id AS TEXT),
              NULLIF(c.business_account_id, ''),
              p.connection_mp_ref
          ) = ?
      )
),
bank_orders AS (
    SELECT
        business_id,
        bank_order_id,
        MIN(payment_date) AS payment_date,
        MAX(bank_sum) AS bank_sum
    FROM monetary
    GROUP BY business_id, bank_order_id
    HAVING MAX(bank_sum) > 0
),
order_base AS (
    SELECT
        m.business_id,
        m.bank_order_id,
        m.order_id,
        SUM(m.transaction_sum) AS direct_net,
        SUM(
            CASE
                WHEN m.transaction_source LIKE 'Плат%ж покупателя'
                     AND m.transaction_sum > 0
                THEN m.transaction_sum
                ELSE 0
            END
        ) AS buyer_payment
    FROM monetary m
    JOIN bank_orders b
      ON b.business_id = m.business_id
     AND b.bank_order_id = m.bank_order_id
    WHERE m.order_id IS NOT NULL
    GROUP BY m.business_id, m.bank_order_id, m.order_id
),
order_weighted AS (
    SELECT
        o.business_id,
        o.bank_order_id,
        o.order_id,
        o.direct_net,
        o.buyer_payment,
        CASE
            WHEN o.buyer_payment > 0 THEN o.buyer_payment
            WHEN o.direct_net > 0 THEN o.direct_net
            ELSE 0
        END AS allocation_weight,
        SUM(o.direct_net) OVER (
            PARTITION BY o.business_id, o.bank_order_id
        ) AS bank_direct_net
    FROM order_base o
),
order_calc AS (
    SELECT
        w.business_id,
        w.bank_order_id,
        w.order_id,
        w.direct_net,
        w.buyer_payment,
        w.allocation_weight,
        w.bank_direct_net,
        b.payment_date,
        b.bank_sum,
        b.bank_sum - w.bank_direct_net AS residual,
        SUM(w.allocation_weight) OVER (
            PARTITION BY w.business_id, w.bank_order_id
        ) AS total_weight
    FROM order_weighted w
    JOIN bank_orders b
      ON b.business_id = w.business_id
     AND b.bank_order_id = w.bank_order_id
),
order_rounded AS (
    SELECT
        c.business_id,
        c.bank_order_id,
        c.order_id,
        c.direct_net,
        c.buyer_payment,
        c.allocation_weight,
        c.bank_direct_net,
        c.payment_date,
        c.bank_sum,
        c.residual,
        c.total_weight,
        CASE
            WHEN c.total_weight > 0
            THEN ROUND(c.residual * c.allocation_weight / c.total_weight, 2)
            ELSE 0
        END AS allocation_rounded,
        ROW_NUMBER() OVER (
            PARTITION BY c.business_id, c.bank_order_id
            ORDER BY c.order_id DESC
        ) AS balance_row
    FROM order_calc c
),
order_balanced AS (
    SELECT
        r.business_id,
        r.bank_order_id,
        r.order_id,
        r.direct_net,
        r.buyer_payment,
        r.allocation_weight,
        r.bank_direct_net,
        r.payment_date,
        r.bank_sum,
        r.residual,
        r.total_weight,
        r.allocation_rounded,
        r.balance_row,
        SUM(r.allocation_rounded) OVER (
            PARTITION BY r.business_id, r.bank_order_id
        ) AS rounded_total
    FROM order_rounded r
),
order_final AS (
    SELECT
        business_id,
        bank_order_id,
        order_id,
        direct_net,
        CASE
            WHEN total_weight <= 0 THEN 0
            WHEN balance_row = 1
            THEN allocation_rounded + (residual - rounded_total)
            ELSE allocation_rounded
        END AS allocated_shared,
        direct_net +
        CASE
            WHEN total_weight <= 0 THEN 0
            WHEN balance_row = 1
            THEN allocation_rounded + (residual - rounded_total)
            ELSE allocation_rounded
        END AS final_payment,
        total_weight
    FROM order_balanced
),
bank_result AS (
    SELECT
        b.business_id,
        b.bank_order_id,
        b.payment_date,
        b.bank_sum,
        COUNT(f.order_id) AS order_count,
        COALESCE(SUM(f.direct_net), 0) AS direct_net,
        COALESCE(SUM(f.allocated_shared), 0) AS allocated_shared,
        COALESCE(SUM(f.final_payment), 0) AS final_payment,
        CASE
            WHEN COALESCE(MAX(f.total_weight), 0) > 0 THEN 0
            ELSE b.bank_sum - COALESCE(SUM(f.direct_net), 0)
        END AS unallocated
    FROM bank_orders b
    LEFT JOIN order_final f
      ON f.business_id = b.business_id
     AND f.bank_order_id = b.bank_order_id
    GROUP BY b.business_id, b.bank_order_id, b.payment_date, b.bank_sum
)
SELECT
    payment_date,
    ROUND(SUM(bank_sum), 2) AS bank_sum,
    SUM(order_count) AS order_count,
    ROUND(SUM(direct_net), 2) AS direct_net,
    ROUND(SUM(allocated_shared), 2) AS allocated_shared,
    ROUND(SUM(final_payment), 2) AS final_payment,
    ROUND(SUM(unallocated), 2) AS unallocated,
    (
        SELECT COUNT(DISTINCT CAST(business_id AS TEXT) || ':' || order_id)
        FROM order_final
    ) AS period_order_count
FROM bank_result
GROUP BY payment_date
ORDER BY payment_date
