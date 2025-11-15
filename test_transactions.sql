-- Проверяем транзакции для конкретного posting_number
SELECT 
    posting_number,
    COUNT(*) as transaction_count,
    GROUP_CONCAT(operation_id) as operation_ids
FROM a014_ozon_transactions 
WHERE is_deleted = 0
GROUP BY posting_number
HAVING posting_number LIKE '%0147833718-0178%';

-- Показываем все posting_number с транзакциями
SELECT DISTINCT posting_number 
FROM a014_ozon_transactions 
WHERE is_deleted = 0
ORDER BY posting_number
LIMIT 10;



