ALTER TABLE a012_wb_sales
    ADD COLUMN prod_cost_problem INTEGER NOT NULL DEFAULT 0;

ALTER TABLE a012_wb_sales
    ADD COLUMN prod_cost_status TEXT;

ALTER TABLE a012_wb_sales
    ADD COLUMN prod_cost_problem_message TEXT;

ALTER TABLE a012_wb_sales
    ADD COLUMN prod_cost_resolved_total REAL;

CREATE INDEX IF NOT EXISTS idx_a012_prod_cost_problem
    ON a012_wb_sales(prod_cost_problem);
