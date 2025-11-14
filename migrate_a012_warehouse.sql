-- Migration to add warehouse_json column to a012_wb_sales table
-- This migration adds support for storing warehouse information for Wildberries sales

-- Add warehouse_json column to store warehouse data
ALTER TABLE a012_wb_sales ADD COLUMN warehouse_json TEXT NOT NULL DEFAULT '{"warehouse_name":null,"warehouse_type":null}';

-- Update existing records with default warehouse data
UPDATE a012_wb_sales 
SET warehouse_json = '{"warehouse_name":null,"warehouse_type":null}'
WHERE warehouse_json = '{"warehouse_name":null,"warehouse_type":null}';

-- Migration complete
SELECT 'Migration completed successfully: Added warehouse_json column to a012_wb_sales table' as result;

