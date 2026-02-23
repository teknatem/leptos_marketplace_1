-- Migration: add enrichment fields to p908_wb_goods_prices
-- ext_nomenklature_ref: resolved UUID from a004_nomenclature (base_ref or own id)
-- dealer_price_ut: dealer price from p906_nomenclature_prices
-- margin_pro: (discounted_price - dealer_price_ut) / dealer_price_ut * 100

ALTER TABLE p908_wb_goods_prices ADD COLUMN ext_nomenklature_ref TEXT;
ALTER TABLE p908_wb_goods_prices ADD COLUMN dealer_price_ut REAL;
ALTER TABLE p908_wb_goods_prices ADD COLUMN margin_pro REAL;

CREATE INDEX IF NOT EXISTS idx_p908_ext_nom_ref ON p908_wb_goods_prices (ext_nomenklature_ref);
