WITH connection_names AS (
    SELECT
        COALESCE(NULLIF(business_account_id, ''), id) AS business_id,
        GROUP_CONCAT(DISTINCT description) AS shops,
        COUNT(*) AS connections_count
    FROM a006_connection_mp
    WHERE is_deleted = 0
    GROUP BY COALESCE(NULLIF(business_account_id, ''), id)
),
payment_cabinets AS (
    SELECT
        COALESCE(
            CAST(p.business_id AS TEXT),
            NULLIF(c.business_account_id, ''),
            p.connection_mp_ref
        ) AS business_id,
        MAX(NULLIF(p.shop_name, '')) AS report_name
    FROM p907_ym_payment_report p
    LEFT JOIN a006_connection_mp c ON c.id = p.connection_mp_ref
    GROUP BY COALESCE(
        CAST(p.business_id AS TEXT),
        NULLIF(c.business_account_id, ''),
        p.connection_mp_ref
    )
)
SELECT
    p.business_id,
    COALESCE(NULLIF(c.shops, ''), NULLIF(p.report_name, ''), 'Кабинет ' || p.business_id) AS name,
    COALESCE(c.shops, p.report_name, '') AS shops,
    COALESCE(c.connections_count, 1) AS connections_count
FROM payment_cabinets p
LEFT JOIN connection_names c ON c.business_id = p.business_id
ORDER BY name
