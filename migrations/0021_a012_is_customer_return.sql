-- Add is_customer_return flag to a012_wb_sales
-- Set based on event_type or negative finished_price for existing rows
ALTER TABLE a012_wb_sales ADD COLUMN is_customer_return BOOLEAN NOT NULL DEFAULT 0;

UPDATE a012_wb_sales
SET is_customer_return = 1
WHERE lower(event_type) = 'return'
   OR (finished_price IS NOT NULL AND finished_price < 0);
