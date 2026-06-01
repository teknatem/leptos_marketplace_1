-- p914: добавляем стабильную ссылку на документ-заказ (uuid + тип документа).
-- order_ref — uuid агрегата заказа (a015_wb_orders / a013_ym_order), nullable:
-- финансовые обороты без заказа (хранение, штрафы, приёмка, прочие удержания)
-- не имеют заказа. order_key (бизнес-ключ srid/order_id) остаётся как был.

ALTER TABLE p914_mp_finance_turnovers ADD COLUMN order_ref TEXT;
ALTER TABLE p914_mp_finance_turnovers ADD COLUMN order_registrator_type TEXT;

CREATE INDEX idx_p914_order_ref ON p914_mp_finance_turnovers(order_ref);
