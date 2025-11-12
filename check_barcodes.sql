-- Проверка данных о штрихкодах

-- 1. Количество штрихкодов по источникам
SELECT source, COUNT(*) as count,
       SUM(CASE WHEN nomenclature_ref IS NULL THEN 1 ELSE 0 END) as without_nomenclature
FROM p901_nomenclature_barcodes
WHERE is_active = 1
GROUP BY source;

-- 2. Последние 10 товаров маркетплейса с их штрихкодами
SELECT
    id,
    marketplace_id,
    marketplace_sku,
    barcode,
    art,
    product_name,
    nomenclature_id
FROM a007_marketplace_product
WHERE marketplace_id = 'YM'
ORDER BY updated_at DESC
LIMIT 10;

-- 3. Последние 10 штрихкодов из YM
SELECT
    barcode,
    source,
    nomenclature_ref,
    article,
    created_at
FROM p901_nomenclature_barcodes
WHERE source = 'YM'
ORDER BY created_at DESC
LIMIT 10;
