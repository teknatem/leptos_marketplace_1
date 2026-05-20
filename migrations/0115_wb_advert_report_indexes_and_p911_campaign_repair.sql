-- Support d404_wb_advert_report aggregation and repair p911 campaign codes for legacy refs.

UPDATE p911_wb_advert_by_items
SET wb_advert_campaign_code = COALESCE((
    SELECT CAST(a.advert_id AS TEXT)
    FROM a026_wb_advert_daily a
    WHERE a.id = REPLACE(p911_wb_advert_by_items.registrator_ref, 'a026:', '')
), '')
WHERE COALESCE(wb_advert_campaign_code, '') = ''
  AND registrator_ref LIKE 'a026:%';

CREATE INDEX IF NOT EXISTS idx_p913_advert_report
    ON p913_wb_advert_order_attr (
        connection_mp_ref,
        entry_date,
        wb_advert_campaign_code,
        nomenclature_ref,
        order_key,
        turnover_code
    );

CREATE INDEX IF NOT EXISTS idx_p911_advert_report
    ON p911_wb_advert_by_items (
        connection_mp_ref,
        entry_date,
        wb_advert_campaign_code,
        nomenclature_ref,
        turnover_code
    );
