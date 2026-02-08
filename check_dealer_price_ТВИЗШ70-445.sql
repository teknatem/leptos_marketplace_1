-- SQL запросы для диагностики проблемы с дилерской ценой для артикула ТВИЗШ70-445

-- 1. Найти номенклатуру с артикулом ТВИЗШ70-445
SELECT 
    id,
    code,
    description,
    article,
    base_nomenclature_ref
FROM a004_nomenclature
WHERE article = 'ТВИЗШ70-445'
ORDER BY created_at DESC;

-- 2. Найти marketplace_product с артикулом ТВИЗШ70-445
SELECT 
    id,
    code,
    description,
    marketplace_sku,
    article,
    nomenclature_ref
FROM a007_marketplace_product
WHERE article = 'ТВИЗШ70-445' OR marketplace_sku = 'ТВИЗШ70-445'
ORDER BY last_update DESC;

-- 3. Найти WB Sales с артикулом ТВИЗШ70-445
SELECT 
    id,
    document_no,
    sale_date,
    supplier_article,
    product_name,
    qty,
    dealer_price_ut,
    marketplace_product_ref,
    nomenclature_ref
FROM a012_wb_sales
WHERE supplier_article = 'ТВИЗШ70-445'
ORDER BY sale_date DESC
LIMIT 10;

-- 4. Проверить цены для номенклатуры из пункта 1
-- (замените NOMENCLATURE_ID на ID из результата запроса 1)
SELECT 
    period,
    price,
    created_at,
    updated_at
FROM p906_nomenclature_prices
WHERE nomenclature_ref IN (
    SELECT id FROM a004_nomenclature WHERE article = 'ТВИЗШ70-445'
)
ORDER BY period DESC
LIMIT 20;

-- 5. Проверить цены для base_nomenclature_ref (если он есть)
-- (замените BASE_NOMENCLATURE_ID на base_nomenclature_ref из результата запроса 1)
SELECT 
    period,
    price,
    created_at,
    updated_at
FROM p906_nomenclature_prices
WHERE nomenclature_ref IN (
    SELECT base_nomenclature_ref 
    FROM a004_nomenclature 
    WHERE article = 'ТВИЗШ70-445' 
      AND base_nomenclature_ref IS NOT NULL 
      AND base_nomenclature_ref != ''
      AND base_nomenclature_ref != '00000000-0000-0000-0000-000000000000'
)
ORDER BY period DESC
LIMIT 20;

-- 6. Объединенный запрос для диагностики
SELECT 
    ws.id AS wb_sales_id,
    ws.document_no,
    ws.sale_date,
    ws.supplier_article,
    ws.dealer_price_ut,
    ws.nomenclature_ref,
    n.article AS nomenclature_article,
    n.description AS nomenclature_name,
    n.base_nomenclature_ref,
    (SELECT COUNT(*) FROM p906_nomenclature_prices WHERE nomenclature_ref = ws.nomenclature_ref) AS price_count_for_nom,
    (SELECT COUNT(*) FROM p906_nomenclature_prices WHERE nomenclature_ref = n.base_nomenclature_ref) AS price_count_for_base
FROM a012_wb_sales ws
LEFT JOIN a004_nomenclature n ON ws.nomenclature_ref = n.id
WHERE ws.supplier_article = 'ТВИЗШ70-445'
ORDER BY ws.sale_date DESC
LIMIT 10;
