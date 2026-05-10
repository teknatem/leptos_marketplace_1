-- Remove legacy WB advert daily documents created before campaign granularity.
-- Since 0082, a026 documents are keyed by (connection_id, document_date, advert_id);
-- rows with advert_id = 0 represent old non-campaign documents and must not remain.

DELETE FROM sys_general_ledger
WHERE registrator_type = 'a026_wb_advert_daily'
  AND (
    registrator_ref IN (
      SELECT id
      FROM a026_wb_advert_daily
      WHERE COALESCE(advert_id, 0) = 0
    )
    OR registrator_ref IN (
      SELECT 'a026:' || id
      FROM a026_wb_advert_daily
      WHERE COALESCE(advert_id, 0) = 0
    )
  );

DELETE FROM p911_wb_advert_by_items
WHERE registrator_type = 'a026_wb_advert_daily'
  AND registrator_ref IN (
    SELECT 'a026:' || id
    FROM a026_wb_advert_daily
    WHERE COALESCE(advert_id, 0) = 0
  );

DELETE FROM a026_wb_advert_daily
WHERE COALESCE(advert_id, 0) = 0;
