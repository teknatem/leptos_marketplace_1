-- 0005: Add dealer price and margin fields to a013_ym_order
-- dealer_price_ut per line item (Дилерская цена УТ)
-- total_dealer_amount and margin_pro aggregated at document level

ALTER TABLE a013_ym_order_items ADD COLUMN dealer_price_ut REAL;

ALTER TABLE a013_ym_order ADD COLUMN total_dealer_amount REAL;
ALTER TABLE a013_ym_order ADD COLUMN margin_pro REAL;
