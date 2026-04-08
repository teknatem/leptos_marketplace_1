ALTER TABLE p903_wb_finance_report
    ADD COLUMN a004_nomenclature_ref TEXT;

UPDATE p903_wb_finance_report
SET a004_nomenclature_ref = (
    SELECT mp.nomenclature_ref
    FROM a007_marketplace_product mp
    WHERE mp.connection_mp_ref = p903_wb_finance_report.connection_mp_ref
      AND mp.marketplace_sku = CAST(p903_wb_finance_report.nm_id AS TEXT)
      AND mp.is_deleted = 0
    LIMIT 1
)
WHERE nm_id IS NOT NULL
  AND (a004_nomenclature_ref IS NULL OR TRIM(a004_nomenclature_ref) = '');

UPDATE p903_wb_finance_report
SET a004_nomenclature_ref = (
    SELECT p908.ext_nomenklature_ref
    FROM p908_wb_goods_prices p908
    WHERE p908.connection_mp_ref = p903_wb_finance_report.connection_mp_ref
      AND p908.nm_id = p903_wb_finance_report.nm_id
    LIMIT 1
)
WHERE nm_id IS NOT NULL
  AND (a004_nomenclature_ref IS NULL OR TRIM(a004_nomenclature_ref) = '');

CREATE INDEX IF NOT EXISTS idx_p903_a004_nomenclature_ref
    ON p903_wb_finance_report (a004_nomenclature_ref);
