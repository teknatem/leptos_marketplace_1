-- Migration: Add financial fields (plan/fact) to a012_wb_sales
-- Adds: is_fact, sell_out_plan/fact, acquiring_fee_plan/fact, 
--       other_fee_plan/fact, supplier_payout_plan/fact, profit_plan/fact
-- Date: 2026-01-25

-- Add is_fact column (boolean stored as INTEGER)
ALTER TABLE a012_wb_sales ADD COLUMN is_fact INTEGER;

-- Add sell_out fields (plan/fact)
ALTER TABLE a012_wb_sales ADD COLUMN sell_out_plan REAL;
ALTER TABLE a012_wb_sales ADD COLUMN sell_out_fact REAL;

-- Add acquiring_fee fields (plan/fact)
ALTER TABLE a012_wb_sales ADD COLUMN acquiring_fee_plan REAL;
ALTER TABLE a012_wb_sales ADD COLUMN acquiring_fee_fact REAL;

-- Add other_fee fields (plan/fact)
ALTER TABLE a012_wb_sales ADD COLUMN other_fee_plan REAL;
ALTER TABLE a012_wb_sales ADD COLUMN other_fee_fact REAL;

-- Add supplier_payout fields (plan/fact)
ALTER TABLE a012_wb_sales ADD COLUMN supplier_payout_plan REAL;
ALTER TABLE a012_wb_sales ADD COLUMN supplier_payout_fact REAL;

-- Add profit fields (plan/fact)
ALTER TABLE a012_wb_sales ADD COLUMN profit_plan REAL;
ALTER TABLE a012_wb_sales ADD COLUMN profit_fact REAL;

-- Add cost of production field
ALTER TABLE a012_wb_sales ADD COLUMN cost_of_production REAL;

-- Add commission fields (plan/fact)
ALTER TABLE a012_wb_sales ADD COLUMN commission_plan REAL;
ALTER TABLE a012_wb_sales ADD COLUMN commission_fact REAL;

-- Verification queries (run manually after migration):

-- Check that columns were added:
-- PRAGMA table_info(a012_wb_sales);

-- Check column count (should show all new columns):
-- SELECT COUNT(*) as column_count FROM pragma_table_info('a012_wb_sales');

-- Sample data check:
-- SELECT id, document_no, is_fact, sell_out_plan, sell_out_fact, 
--        acquiring_fee_plan, acquiring_fee_fact, profit_plan, profit_fact,
--        cost_of_production, commission_plan, commission_fact
-- FROM a012_wb_sales 
-- LIMIT 5;
