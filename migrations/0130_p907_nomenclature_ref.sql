-- p907: добавляем производную ссылку nomenclature_ref (по аналогии с p903.a004_nomenclature_ref).
-- Резолвится на этапе проведения из a007 (по marketplace_product_ref) и копируется в p914,
-- чтобы оборот fina нёс ссылку на номенклатуру 1С так же, как WB-ветка.

ALTER TABLE p907_ym_payment_report ADD COLUMN nomenclature_ref TEXT;

CREATE INDEX idx_p907_nomenclature_ref ON p907_ym_payment_report(nomenclature_ref);
