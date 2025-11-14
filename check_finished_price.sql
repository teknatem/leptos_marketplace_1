-- Проверка наличия finished_price в базе данных WB Sales

-- 1. Проверить структуру line_json
SELECT 
    document_no,
    json_extract(line_json, '$.finished_price') as finished_price,
    json_extract(line_json, '$.total_price') as total_price,
    json_extract(line_json, '$.amount_line') as amount_line,
    json_extract(line_json, '$.price_list') as price_list,
    created_at
FROM a012_wb_sales
ORDER BY created_at DESC
LIMIT 10;

-- 2. Подсчитать количество записей с finished_price
SELECT 
    COUNT(*) as total_records,
    SUM(CASE WHEN json_extract(line_json, '$.finished_price') IS NOT NULL THEN 1 ELSE 0 END) as records_with_finished_price,
    SUM(CASE WHEN json_extract(line_json, '$.finished_price') IS NULL THEN 1 ELSE 0 END) as records_without_finished_price
FROM a012_wb_sales;

-- 3. Показать пример полного line_json для проверки структуры
SELECT 
    document_no,
    line_json
FROM a012_wb_sales
ORDER BY created_at DESC
LIMIT 1;

