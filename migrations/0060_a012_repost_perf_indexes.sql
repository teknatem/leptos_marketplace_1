CREATE INDEX IF NOT EXISTS idx_p900_registrator_ref
    ON p900_sales_register(registrator_ref);

CREATE INDEX IF NOT EXISTS idx_p903_srid
    ON p903_wb_finance_report(srid);

CREATE INDEX IF NOT EXISTS idx_a012_repost_chunk
    ON a012_wb_sales(sale_date, connection_id, is_posted, is_deleted);
