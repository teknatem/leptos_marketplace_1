-- Migration: Add WB Sales fields (dealer_price_ut, is_fact) to p900_sales_register
-- Adds: dealer_price_ut (дилерская цена УТ), is_fact (флаг факт/план)
-- Date: 2026-02-09

-- Add dealer_price_ut column (дилерская цена из УТ)
ALTER TABLE p900_sales_register ADD COLUMN dealer_price_ut REAL;

-- Add is_fact column (boolean stored as INTEGER)
ALTER TABLE p900_sales_register ADD COLUMN is_fact INTEGER;

-- Verification queries (run manually after migration):

-- Check that columns were added:
-- PRAGMA table_info(p900_sales_register);

-- Check column count:
-- SELECT COUNT(*) as column_count FROM pragma_table_info('p900_sales_register');

-- Sample data check:
-- SELECT marketplace, document_no, dealer_price_ut, is_fact
-- FROM p900_sales_register 
-- WHERE marketplace = 'WB'
-- LIMIT 5;
