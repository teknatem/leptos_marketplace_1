WITH connection_business AS (
    SELECT
        id AS connection_mp_ref,
        COALESCE(NULLIF(business_account_id, ''), id) AS business_id,
        COALESCE(NULLIF(description, ''), 'Кабинет') AS cabinet_name
    FROM a006_connection_mp
    WHERE is_deleted = 0
),
selected_orders AS (
    SELECT
        cb.business_id,
        o.id AS order_ref,
        o.document_no AS order_id,
        DATE(o.creation_date, '+3 hours') AS order_date,
        COALESCE(o.status_norm, '') AS order_status,
        COALESCE(o.total_amount, 0) AS order_amount
    FROM a013_ym_order o
    JOIN connection_business cb ON cb.connection_mp_ref = o.connection_id
    WHERE o.is_deleted = 0
      AND DATE(o.creation_date, '+3 hours') BETWEEN ? AND ?
      AND (? = '' OR cb.business_id = ?)
),
monetary AS (
    SELECT
        COALESCE(
            CAST(p.business_id AS TEXT),
            NULLIF(c.business_account_id, ''),
            p.connection_mp_ref
        ) AS business_id,
        p.bank_order_id,
        p.order_id,
        COALESCE(p.transaction_sum, 0) AS transaction_sum,
        COALESCE(p.transaction_source, '') AS transaction_source,
        COALESCE(p.bank_sum, 0) AS bank_sum
    FROM p907_ym_payment_report p
    LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
    WHERE p.bank_order_id IS NOT NULL
      AND p.bank_order_date IS NOT NULL
      AND TRIM(p.bank_order_date) <> ''
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
        MAX(bank_sum) AS bank_sum
    FROM monetary
    GROUP BY business_id, bank_order_id
    HAVING MAX(bank_sum) > 0
),
order_base AS (
    SELECT
        m.business_id,
        m.bank_order_id,
        CAST(m.order_id AS TEXT) AS order_id,
        SUM(m.transaction_sum) AS direct_net,
        SUM(
            CASE WHEN m.transaction_sum > 0
                 THEN m.transaction_sum ELSE 0 END
        ) AS order_accruals,
        SUM(
            CASE WHEN m.transaction_sum < 0
                 THEN m.transaction_sum ELSE 0 END
        ) AS order_withholdings,
        SUM(
            CASE
                WHEN m.transaction_source LIKE 'Плат%ж покупателя'
                 AND m.transaction_sum > 0
                THEN m.transaction_sum ELSE 0
            END
        ) AS buyer_payment
    FROM monetary m
    JOIN bank_orders b
      ON b.business_id = m.business_id
     AND b.bank_order_id = m.bank_order_id
    WHERE m.order_id IS NOT NULL
    GROUP BY m.business_id, m.bank_order_id, CAST(m.order_id AS TEXT)
),
order_weighted AS (
    SELECT
        o.business_id,
        o.bank_order_id,
        o.order_id,
        o.direct_net,
        o.order_accruals,
        o.order_withholdings,
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
        w.order_accruals,
        w.order_withholdings,
        w.buyer_payment,
        w.allocation_weight,
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
        c.order_accruals,
        c.order_withholdings,
        c.buyer_payment,
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
        r.order_accruals,
        r.order_withholdings,
        r.buyer_payment,
        r.residual,
        r.total_weight,
        r.allocation_rounded,
        r.balance_row,
        SUM(r.allocation_rounded) OVER (
            PARTITION BY r.business_id, r.bank_order_id
        ) AS rounded_total
    FROM order_rounded r
),
order_payments AS (
    SELECT
        business_id,
        order_id,
        SUM(order_accruals) AS order_accruals,
        SUM(order_withholdings) AS order_withholdings,
        SUM(buyer_payment) AS buyer_payment,
        SUM(direct_net) AS direct_net,
        SUM(
            CASE
                WHEN total_weight <= 0 THEN 0
                WHEN balance_row = 1
                THEN allocation_rounded + (residual - rounded_total)
                ELSE allocation_rounded
            END
        ) AS allocated_shared,
        SUM(
            direct_net +
            CASE
                WHEN total_weight <= 0 THEN 0
                WHEN balance_row = 1
                THEN allocation_rounded + (residual - rounded_total)
                ELSE allocation_rounded
            END
        ) AS final_payment,
        COUNT(DISTINCT bank_order_id) AS payment_orders
    FROM order_balanced
    GROUP BY business_id, order_id
),
order_events AS (
    SELECT
        so.business_id,
        so.order_id,
        SUM(
            CASE WHEN e.event_type = 'realization'
                 THEN COALESCE(e.amount, 0) ELSE 0 END
        ) AS realization_amount,
        SUM(
            CASE WHEN e.event_type = 'goods_return'
                 THEN ABS(COALESCE(e.amount, 0)) ELSE 0 END
        ) AS goods_return_amount
    FROM selected_orders so
    LEFT JOIN p915_mp_order_events e
      ON e.order_id = so.order_id
     AND EXISTS (
         SELECT 1
         FROM connection_business ec
         WHERE ec.connection_mp_ref = e.connection_mp_ref
           AND ec.business_id = so.business_id
     )
    GROUP BY so.business_id, so.order_id
),
order_facts AS (
    SELECT
        so.business_id,
        so.order_id,
        so.order_date,
        so.order_status,
        so.order_amount,
        CASE
            WHEN COALESCE(oe.realization_amount, 0) <> 0
            THEN oe.realization_amount
            WHEN so.order_status = 'DELIVERED'
            THEN so.order_amount
            ELSE 0
        END AS realization_amount,
        COALESCE(oe.goods_return_amount, 0) AS goods_return_amount,
        COALESCE(op.order_accruals, 0) AS order_accruals,
        COALESCE(op.order_withholdings, 0) AS order_withholdings,
        COALESCE(op.buyer_payment, 0) AS buyer_payment,
        COALESCE(op.allocated_shared, 0) AS allocated_shared,
        COALESCE(op.final_payment, 0) AS final_payment,
        COALESCE(op.payment_orders, 0) AS payment_orders
    FROM selected_orders so
    LEFT JOIN order_events oe
      ON oe.business_id = so.business_id
     AND oe.order_id = so.order_id
    LEFT JOIN order_payments op
      ON op.business_id = so.business_id
     AND op.order_id = so.order_id
)
SELECT
    order_date AS event_date,
    COUNT(order_id) AS order_count,
    ROUND(SUM(order_amount), 2) AS order_amount,
    SUM(CASE WHEN realization_amount <> 0 THEN 1 ELSE 0 END) AS realized_count,
    ROUND(SUM(realization_amount), 2) AS realization,
    SUM(CASE WHEN goods_return_amount > 0 THEN 1 ELSE 0 END) AS returned_count,
    ROUND(-SUM(goods_return_amount), 2) AS goods_return,
    SUM(CASE WHEN order_status = 'CANCELLED' THEN 1 ELSE 0 END) AS cancelled_count,
    ROUND(-SUM(
        CASE WHEN order_status = 'CANCELLED'
             THEN order_amount ELSE 0 END
    ), 2) AS cancellation,
    SUM(CASE WHEN payment_orders > 0 THEN 1 ELSE 0 END) AS paid_count,
    SUM(
        CASE
            WHEN order_status = 'DELIVERED'
             AND goods_return_amount = 0
             AND payment_orders > 0
            THEN 1 ELSE 0
        END
    ) AS settlement_paid_count,
    ROUND(SUM(
        CASE
            WHEN order_status = 'DELIVERED'
             AND goods_return_amount = 0
             AND payment_orders > 0
            THEN final_payment ELSE 0
        END
    ), 2) AS settlement_paid_amount,
    ROUND(SUM(final_payment), 2) AS ym_payment,
    ROUND(SUM(order_accruals), 2) AS order_accruals,
    ROUND(SUM(order_withholdings), 2) AS order_withholdings,
    ROUND(SUM(
        CASE WHEN allocated_shared > 0
             THEN allocated_shared ELSE 0 END
    ), 2) AS common_accruals,
    ROUND(SUM(
        CASE WHEN allocated_shared < 0
             THEN allocated_shared ELSE 0 END
    ), 2) AS common_withholdings,
    ROUND(SUM(
        CASE
            WHEN final_payment - buyer_payment + goods_return_amount < 0
            THEN final_payment - buyer_payment + goods_return_amount
            ELSE 0
        END
    ), 2) AS other_expenses
FROM order_facts
GROUP BY order_date
ORDER BY event_date
