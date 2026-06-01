-- p907: добавляем производные ссылки (по аналогии с p903), резолвящиеся на
-- первом этапе проведения (если ещё не заполнены) и копирующиеся затем в p914.
-- marketplace_product_ref — uuid a007_marketplace_product (по shop_sku).
-- marketplace_order_ref   — uuid a013_ym_order (по order_id).

ALTER TABLE p907_ym_payment_report ADD COLUMN marketplace_product_ref TEXT;
ALTER TABLE p907_ym_payment_report ADD COLUMN marketplace_order_ref TEXT;

CREATE INDEX idx_p907_marketplace_product_ref ON p907_ym_payment_report(marketplace_product_ref);
CREATE INDEX idx_p907_marketplace_order_ref ON p907_ym_payment_report(marketplace_order_ref);
