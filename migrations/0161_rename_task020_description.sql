-- Переименование: «Снимки товаров WB» → «Данные по товарам WB».
-- Строка задачи task020 была засеяна миграцией 0160 со старым термином в description;
-- меняем текст отдельной forward-only миграцией (править применённую 0160 нельзя — checksum).
-- Подпись типа задачи в UI берётся из кода (TaskMetadata.display_name); здесь — только seed-строка.
UPDATE sys_tasks
SET description = 'WB Данные по товарам — остатки и рейтинги (раз в день). Замените connection_id на UUID WB-кабинета.',
    updated_at = datetime('now')
WHERE id = 'a1b2c3d4-e5f6-7890-abcd-ef1234567820'
  AND task_type = 'task020_wb_product_snapshot'
  AND description = 'WB Снимки товаров — остатки и рейтинги (раз в день). Замените connection_id на UUID WB-кабинета.';
