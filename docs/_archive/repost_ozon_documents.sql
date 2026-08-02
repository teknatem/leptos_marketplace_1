-- Скрипт для перепроведения OZON FBO и OZON Returns документов
-- Это обновит is_posted на 0, чтобы документы можно было провести заново

-- 1. OZON FBO
SELECT '=== OZON FBO ===' as info;

-- Проверяем количество документов FBO
SELECT 
    COUNT(*) as total_fbo_docs,
    SUM(CASE WHEN is_posted = 1 THEN 1 ELSE 0 END) as posted,
    SUM(CASE WHEN is_posted = 0 THEN 1 ELSE 0 END) as not_posted
FROM a011_ozon_fbo_posting;

-- Отменяем проведение всех FBO документов
UPDATE a011_ozon_fbo_posting 
SET is_posted = 0
WHERE is_posted = 1;

-- Проверяем результат FBO
SELECT 
    COUNT(*) as total_fbo_docs,
    SUM(CASE WHEN is_posted = 1 THEN 1 ELSE 0 END) as posted,
    SUM(CASE WHEN is_posted = 0 THEN 1 ELSE 0 END) as not_posted
FROM a011_ozon_fbo_posting;

-- 2. OZON Returns
SELECT '=== OZON Returns ===' as info;

-- Проверяем количество документов Returns
SELECT 
    COUNT(*) as total_returns,
    SUM(CASE WHEN is_posted = 1 THEN 1 ELSE 0 END) as posted,
    SUM(CASE WHEN is_posted = 0 THEN 1 ELSE 0 END) as not_posted
FROM a009_ozon_returns;

-- Отменяем проведение всех Returns документов
UPDATE a009_ozon_returns 
SET is_posted = 0
WHERE is_posted = 1;

-- Проверяем результат Returns
SELECT 
    COUNT(*) as total_returns,
    SUM(CASE WHEN is_posted = 1 THEN 1 ELSE 0 END) as posted,
    SUM(CASE WHEN is_posted = 0 THEN 1 ELSE 0 END) as not_posted
FROM a009_ozon_returns;

-- Информация: после выполнения этого скрипта нужно провести документы через API или UI
