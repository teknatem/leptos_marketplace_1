CREATE INDEX IF NOT EXISTS idx_a007_connection_marketplace_sku
    ON a007_marketplace_product (connection_mp_ref, marketplace_sku);

CREATE INDEX IF NOT EXISTS idx_a007_connection_article
    ON a007_marketplace_product (connection_mp_ref, article);

UPDATE p903_wb_finance_report
SET a004_nomenclature_ref = (
    SELECT mp.nomenclature_ref
    FROM a007_marketplace_product mp
    WHERE mp.connection_mp_ref = p903_wb_finance_report.connection_mp_ref
      AND mp.article = p903_wb_finance_report.sa_name
      AND mp.is_deleted = 0
    GROUP BY mp.connection_mp_ref, mp.article
    HAVING COUNT(*) = 1
    LIMIT 1
)
WHERE COALESCE(TRIM(sa_name), '') <> ''
  AND (a004_nomenclature_ref IS NULL OR TRIM(a004_nomenclature_ref) = '');
