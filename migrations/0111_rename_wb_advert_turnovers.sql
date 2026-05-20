-- Rename WB advertising turnover codes to the advert_clicks_* naming scheme.

UPDATE sys_general_ledger
SET turnover_code = CASE turnover_code
    WHEN 'advert_reserve' THEN 'advert_clicks_order_accrual'
    WHEN 'advert_expense' THEN 'advert_clicks_order_expense'
    WHEN 'advertising_allocated' THEN 'advert_clicks_no_order'
    ELSE turnover_code
END
WHERE turnover_code IN ('advert_reserve', 'advert_expense', 'advertising_allocated');

UPDATE p913_wb_advert_order_attr
SET turnover_code = CASE turnover_code
    WHEN 'advert_reserve' THEN 'advert_clicks_order_accrual'
    WHEN 'advert_expense' THEN 'advert_clicks_order_expense'
    ELSE turnover_code
END
WHERE turnover_code IN ('advert_reserve', 'advert_expense');

UPDATE p911_wb_advert_by_items
SET turnover_code = 'advert_clicks_no_order'
WHERE turnover_code = 'advertising_allocated';

UPDATE a024_bi_indicator
SET params_json = REPLACE(params_json, 'advertising_allocated', 'advert_clicks_no_order'),
    comment = REPLACE(comment, 'advertising_allocated', 'advert_clicks_no_order'),
    explanation = REPLACE(explanation, 'advertising_allocated', 'advert_clicks_no_order'),
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE params_json LIKE '%advertising_allocated%'
   OR comment LIKE '%advertising_allocated%'
   OR explanation LIKE '%advertising_allocated%';
