WITH connection_business AS (
    SELECT
        id AS connection_mp_ref,
        COALESCE(NULLIF(business_account_id, ''), id) AS business_id
    FROM a006_connection_mp
    WHERE is_deleted = 0
),
return_orders AS (
    SELECT
        cb.business_id,
        e.order_id
    FROM p915_mp_order_events e
    JOIN connection_business cb ON cb.connection_mp_ref = e.connection_mp_ref
    WHERE e.event_type = 'goods_return'
    GROUP BY cb.business_id, e.order_id
)
SELECT
    COUNT(DISTINCT cb.business_id || ':' || o.document_no) AS due_order_count,
    ROUND(COALESCE(SUM(COALESCE(o.total_amount, 0)), 0), 2) AS due_amount
FROM a013_ym_order o
JOIN connection_business cb ON cb.connection_mp_ref = o.connection_id
LEFT JOIN return_orders ro
  ON ro.business_id = cb.business_id
 AND ro.order_id = o.document_no
WHERE o.is_deleted = 0
  AND o.status_norm = 'DELIVERED'
  AND ro.order_id IS NULL
  AND DATE(o.creation_date, '+3 hours') BETWEEN ? AND ?
  AND (? = '' OR cb.business_id = ?)
