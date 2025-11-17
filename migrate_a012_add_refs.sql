-- Миграция: добавление ссылок на номенклатуру и товар МП в a012_wb_sales
-- Дата: 2025-11-16

-- Добавляем поле nomenclature_ref - ссылка на a004_nomenclature
ALTER TABLE a012_wb_sales ADD COLUMN nomenclature_ref TEXT NULL;

-- Добавляем поле marketplace_product_ref - ссылка на a007_marketplace_product
ALTER TABLE a012_wb_sales ADD COLUMN marketplace_product_ref TEXT NULL;

-- Создаем индексы для оптимизации поиска
CREATE INDEX IF NOT EXISTS idx_a012_wb_sales_nomenclature_ref ON a012_wb_sales(nomenclature_ref);
CREATE INDEX IF NOT EXISTS idx_a012_wb_sales_marketplace_product_ref ON a012_wb_sales(marketplace_product_ref);

