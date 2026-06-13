-- a013_ym_order: денормализованный столбец fulfillment_type (placementType кампании:
-- FBS / FBY / DBS / LAAS). Измерение, заменяющее «магазин» в аналитике YM при модели
-- «подключение = бизнес». Зеркало header_json.$.fulfillment_type; заполняется при каждом
-- сохранении документа. Бэкфилл — для уже импортированных заказов, где значение есть в JSON.

ALTER TABLE a013_ym_order ADD COLUMN fulfillment_type TEXT;

UPDATE a013_ym_order
SET fulfillment_type = json_extract(header_json, '$.fulfillment_type')
WHERE fulfillment_type IS NULL
  AND json_extract(header_json, '$.fulfillment_type') IS NOT NULL;

CREATE INDEX idx_a013_fulfillment_type ON a013_ym_order(fulfillment_type);
