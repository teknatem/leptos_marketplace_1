-- p903: добавляем производные ссылки, резолвящиеся на первом этапе проведения
-- (если ещё не заполнены) и копирующиеся затем в p914.
-- marketplace_product_ref — uuid a007_marketplace_product (по nm_id/артикулу).
-- marketplace_order_ref   — uuid a015_wb_orders (по srid).

ALTER TABLE p903_wb_finance_report ADD COLUMN marketplace_product_ref TEXT;
ALTER TABLE p903_wb_finance_report ADD COLUMN marketplace_order_ref TEXT;

CREATE INDEX idx_p903_marketplace_product_ref ON p903_wb_finance_report(marketplace_product_ref);
CREATE INDEX idx_p903_marketplace_order_ref ON p903_wb_finance_report(marketplace_order_ref);
