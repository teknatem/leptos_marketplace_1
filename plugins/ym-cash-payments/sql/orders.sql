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
        cb.cabinet_name,
        o.id AS order_ref,
        o.document_no AS order_id,
        DATE(o.creation_date, '+3 hours') AS order_date,
        COALESCE(DATE(o.delivery_date, '+3 hours'), '') AS delivery_date,
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
        SUBSTR(p.bank_order_date, 1, 10) AS payment_date,
        CAST(p.order_id AS TEXT) AS order_id,
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
                THEN m.transaction_sum ELSE 0
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
        b.payment_date,
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
        c.payment_date,
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
        r.payment_date,
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
        MIN(payment_date) AS first_payment_date,
        MAX(payment_date) AS last_payment_date,
        GROUP_CONCAT(DISTINCT CAST(bank_order_id AS TEXT)) AS bank_order_ids,
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
events_with_business AS (
    SELECT
        cb.business_id,
        e.order_id,
        e.event_date,
        e.event_type,
        COALESCE(e.amount, 0) AS amount
    FROM p915_mp_order_events e
    JOIN connection_business cb ON cb.connection_mp_ref = e.connection_mp_ref
),
order_events AS (
    SELECT
        so.business_id,
        so.order_id,
        MAX(
            CASE WHEN e.event_type = 'realization'
                 THEN e.event_date ELSE '' END
        ) AS realization_date,
        SUM(
            CASE WHEN e.event_type = 'realization'
                 THEN e.amount ELSE 0 END
        ) AS realization_amount,
        SUM(
            CASE WHEN e.event_type = 'goods_return'
                 THEN ABS(e.amount) ELSE 0 END
        ) AS return_amount
    FROM selected_orders so
    LEFT JOIN events_with_business e
      ON e.business_id = so.business_id
     AND e.order_id = so.order_id
    GROUP BY so.business_id, so.order_id
),
final_rows AS (
    SELECT
        so.business_id,
        so.cabinet_name,
        so.order_date,
        so.order_id,
        so.order_ref,
        so.order_status,
        so.order_amount,
        CASE
            WHEN COALESCE(oe.realization_date, '') <> '' THEN oe.realization_date
            WHEN so.order_status = 'DELIVERED' THEN so.delivery_date
            ELSE ''
        END AS realization_date,
        CASE
            WHEN COALESCE(oe.realization_amount, 0) <> 0 THEN oe.realization_amount
            WHEN so.order_status = 'DELIVERED' THEN so.order_amount
            ELSE 0
        END AS realization_amount,
        COALESCE(oe.return_amount, 0) AS return_amount,
        COALESCE(op.first_payment_date, '') AS first_payment_date,
        COALESCE(op.last_payment_date, '') AS payment_date,
        COALESCE(op.bank_order_ids, '') AS bank_order_id,
        COALESCE(op.payment_orders, 0) AS payment_orders,
        COALESCE(op.buyer_payment, 0) AS buyer_payment,
        COALESCE(op.direct_net, 0) AS direct_net,
        COALESCE(op.allocated_shared, 0) AS allocated_shared,
        COALESCE(op.final_payment, 0) AS final_payment,
        0 AS unallocated
    FROM selected_orders so
    LEFT JOIN order_events oe
      ON oe.business_id = so.business_id
     AND oe.order_id = so.order_id
    LEFT JOIN order_payments op
      ON op.business_id = so.business_id
     AND op.order_id = so.order_id
    WHERE (? = '' OR so.order_date = ?)
),
counted AS (
    SELECT
        f.business_id,
        f.cabinet_name,
        f.order_date,
        f.order_id,
        f.order_ref,
        f.order_status,
        f.order_amount,
        f.realization_date,
        f.realization_amount,
        f.return_amount,
        f.first_payment_date,
        f.payment_date,
        f.bank_order_id,
        f.payment_orders,
        f.buyer_payment,
        f.direct_net,
        f.allocated_shared,
        f.final_payment,
        f.unallocated,
        COUNT(*) OVER () AS total_rows
    FROM final_rows f
)
SELECT
    business_id,
    cabinet_name,
    order_date,
    order_id,
    order_ref,
    order_status,
    order_amount,
    realization_date,
    realization_amount,
    return_amount,
    first_payment_date,
    payment_date,
    bank_order_id,
    payment_orders,
    buyer_payment,
    direct_net,
    allocated_shared,
    final_payment,
    unallocated,
    total_rows
FROM counted
ORDER BY
    CASE WHEN ? = 'asc' THEN
        CASE ?
            WHEN 'order_date' THEN order_date
            WHEN 'order_id' THEN order_id
            WHEN 'order_status' THEN order_status
            WHEN 'order_amount' THEN order_amount
            WHEN 'realization_date' THEN realization_date
            WHEN 'payment_date' THEN payment_date
            WHEN 'bank_order_id' THEN bank_order_id
            WHEN 'cabinet_name' THEN cabinet_name
            WHEN 'buyer_payment' THEN buyer_payment
            WHEN 'direct_net' THEN direct_net
            WHEN 'allocated_shared' THEN allocated_shared
            WHEN 'final_payment' THEN final_payment
        END
    END ASC,
    CASE WHEN ? = 'desc' THEN
        CASE ?
            WHEN 'order_date' THEN order_date
            WHEN 'order_id' THEN order_id
            WHEN 'order_status' THEN order_status
            WHEN 'order_amount' THEN order_amount
            WHEN 'realization_date' THEN realization_date
            WHEN 'payment_date' THEN payment_date
            WHEN 'bank_order_id' THEN bank_order_id
            WHEN 'cabinet_name' THEN cabinet_name
            WHEN 'buyer_payment' THEN buyer_payment
            WHEN 'direct_net' THEN direct_net
            WHEN 'allocated_shared' THEN allocated_shared
            WHEN 'final_payment' THEN final_payment
        END
    END DESC,
    order_date DESC,
    order_id DESC
LIMIT ? OFFSET ?
