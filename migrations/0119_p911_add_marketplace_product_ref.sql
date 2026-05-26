-- p911: новое измерение marketplace_product_ref (ссылка на a007_marketplace_product).
--
-- Позиции рекламных отчётов (nm_id) ранее могли отсутствовать в a007. Теперь при
-- проведении a026 для каждого nm_id гарантированно создаётся a007 (автоматически,
-- с пометкой в комментарии), а p911-строка ссылается на него через это поле.
--
-- Существующие строки заполняем best-effort через nomenclature_ref: если в том же
-- кабинете ровно один a007 привязан к этой номенклатуре. Остальные строки получат
-- ссылку при перепроведении документа-источника.

ALTER TABLE p911_wb_advert_by_items ADD COLUMN marketplace_product_ref TEXT;

UPDATE p911_wb_advert_by_items
SET marketplace_product_ref = (
    SELECT a.id
    FROM a007_marketplace_product a
    WHERE a.connection_mp_ref = p911_wb_advert_by_items.connection_mp_ref
      AND a.nomenclature_ref = p911_wb_advert_by_items.nomenclature_ref
      AND a.is_deleted = 0
)
WHERE nomenclature_ref IS NOT NULL
  AND (
    SELECT COUNT(*)
    FROM a007_marketplace_product a
    WHERE a.connection_mp_ref = p911_wb_advert_by_items.connection_mp_ref
      AND a.nomenclature_ref = p911_wb_advert_by_items.nomenclature_ref
      AND a.is_deleted = 0
  ) = 1;

CREATE INDEX idx_p911_marketplace_product_ref
    ON p911_wb_advert_by_items (marketplace_product_ref);
