CREATE INDEX IF NOT EXISTS idx_a012_wb_sales_connection_article_nm_id
    ON a012_wb_sales (connection_id, supplier_article, nm_id);

CREATE INDEX IF NOT EXISTS idx_a015_wb_orders_connection_article_nm_id
    ON a015_wb_orders (
        json_extract(header_json, '$.connection_id'),
        json_extract(line_json, '$.supplier_article'),
        json_extract(line_json, '$.nm_id')
    );

WITH wb_mapping AS (
    SELECT
        connection_id AS connection_mp_ref,
        supplier_article AS article,
        CAST(nm_id AS TEXT) AS nm_id_text
    FROM a012_wb_sales
    WHERE supplier_article IS NOT NULL
      AND TRIM(supplier_article) <> ''
      AND nm_id IS NOT NULL
      AND nm_id > 0

    UNION ALL

    SELECT
        json_extract(header_json, '$.connection_id') AS connection_mp_ref,
        json_extract(line_json, '$.supplier_article') AS article,
        CAST(json_extract(line_json, '$.nm_id') AS TEXT) AS nm_id_text
    FROM a015_wb_orders
    WHERE json_extract(line_json, '$.supplier_article') IS NOT NULL
      AND TRIM(json_extract(line_json, '$.supplier_article')) <> ''
      AND CAST(json_extract(line_json, '$.nm_id') AS INTEGER) > 0
),
unique_mapping AS (
    SELECT
        connection_mp_ref,
        article,
        MIN(nm_id_text) AS nm_id_text
    FROM wb_mapping
    GROUP BY connection_mp_ref, article
    HAVING COUNT(DISTINCT nm_id_text) = 1
)
UPDATE a007_marketplace_product
SET marketplace_sku = (
        SELECT unique_mapping.nm_id_text
        FROM unique_mapping
        WHERE unique_mapping.connection_mp_ref = a007_marketplace_product.connection_mp_ref
          AND unique_mapping.article = a007_marketplace_product.article
    ),
    updated_at = datetime('now'),
    version = version + 1
WHERE marketplace_ref = (
        SELECT id
        FROM a005_marketplace
        WHERE code = 'mp-wb'
        LIMIT 1
    )
  AND COALESCE(TRIM(article), '') <> ''
  AND NOT (
        COALESCE(TRIM(marketplace_sku), '') <> ''
        AND TRIM(marketplace_sku) NOT GLOB '*[^0-9]*'
    )
  AND EXISTS (
        SELECT 1
        FROM unique_mapping
        WHERE unique_mapping.connection_mp_ref = a007_marketplace_product.connection_mp_ref
          AND unique_mapping.article = a007_marketplace_product.article
    )
  AND NOT EXISTS (
        SELECT 1
        FROM a007_marketplace_product AS other
        WHERE other.id <> a007_marketplace_product.id
          AND other.is_deleted = 0
          AND other.connection_mp_ref = a007_marketplace_product.connection_mp_ref
          AND other.marketplace_sku = (
                SELECT unique_mapping.nm_id_text
                FROM unique_mapping
                WHERE unique_mapping.connection_mp_ref = a007_marketplace_product.connection_mp_ref
                  AND unique_mapping.article = a007_marketplace_product.article
            )
    );

UPDATE p903_wb_finance_report
SET a004_nomenclature_ref = (
    SELECT mp.nomenclature_ref
    FROM a007_marketplace_product mp
    JOIN a005_marketplace market ON market.id = mp.marketplace_ref
    WHERE market.code = 'mp-wb'
      AND mp.connection_mp_ref = p903_wb_finance_report.connection_mp_ref
      AND mp.marketplace_sku = CAST(p903_wb_finance_report.nm_id AS TEXT)
      AND mp.is_deleted = 0
    GROUP BY mp.connection_mp_ref, mp.marketplace_sku
    HAVING COUNT(*) = 1
    LIMIT 1
)
WHERE (a004_nomenclature_ref IS NULL OR TRIM(a004_nomenclature_ref) = '')
  AND nm_id IS NOT NULL
  AND nm_id > 0;
