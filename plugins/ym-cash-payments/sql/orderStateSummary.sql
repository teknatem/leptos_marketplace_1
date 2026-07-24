WITH connection_business AS (
    SELECT
        id AS connection_mp_ref,
        COALESCE(NULLIF(business_account_id, ''), id) AS business_id
    FROM a006_connection_mp
    WHERE is_deleted = 0
),
selected_orders AS (
    SELECT
        cb.business_id,
        o.id AS order_ref,
        o.document_no AS order_id,
        COALESCE(o.status_norm, '') AS status_norm,
        COALESCE(o.total_amount, 0) AS order_amount
    FROM a013_ym_order o
    JOIN connection_business cb ON cb.connection_mp_ref = o.connection_id
    WHERE o.is_deleted = 0
      AND DATE(o.creation_date, '+3 hours') BETWEEN ? AND ?
      AND (? = '' OR cb.business_id = ?)
),
return_orders AS (
    SELECT
        cb.business_id,
        e.order_id,
        SUM(ABS(COALESCE(e.amount, 0))) AS return_amount
    FROM p915_mp_order_events e
    JOIN connection_business cb ON cb.connection_mp_ref = e.connection_mp_ref
    WHERE e.event_type = 'goods_return'
    GROUP BY cb.business_id, e.order_id
)
SELECT
    'total' AS state_key,
    'Всего' AS state_name,
    COUNT(so.order_id) AS orders_count,
    ROUND(COALESCE(SUM(so.order_amount), 0), 2) AS amount
FROM selected_orders so
UNION ALL
SELECT
    'realized' AS state_key,
    'Реализация' AS state_name,
    COUNT(so.order_id) AS orders_count,
    ROUND(COALESCE(SUM(so.order_amount), 0), 2) AS amount
FROM selected_orders so
LEFT JOIN return_orders ro
  ON ro.business_id = so.business_id
 AND ro.order_id = so.order_id
WHERE so.status_norm = 'DELIVERED'
  AND ro.order_id IS NULL
UNION ALL
SELECT
    'cancelled' AS state_key,
    'Отказы' AS state_name,
    COUNT(so.order_id) AS orders_count,
    ROUND(COALESCE(SUM(so.order_amount), 0), 2) AS amount
FROM selected_orders so
WHERE so.status_norm = 'CANCELLED'
UNION ALL
SELECT
    'returned' AS state_key,
    'Возвраты' AS state_name,
    COUNT(so.order_id) AS orders_count,
    ROUND(COALESCE(SUM(ro.return_amount), 0), 2) AS amount
FROM selected_orders so
JOIN return_orders ro
  ON ro.business_id = so.business_id
 AND ro.order_id = so.order_id
