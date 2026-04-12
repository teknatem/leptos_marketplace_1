-- Move IND-WB-ADS-SPEND to GL-first dv004 and route historical a026 GL rows to p911 detail.

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"advertising_allocated","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Ready-to-use BI indicator on DataView dv004_general_ledger_turnovers with turnover_code=advertising_allocated and layer=oper. Scalar comes from GL, drilldown uses linked p911 details.',
    updated_by = 'system',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-WB-ADS-SPEND';

UPDATE sys_general_ledger
SET
    resource_table = 'p911_wb_advert_by_items',
    resource_field = 'amount',
    resource_sign = 1
WHERE registrator_type = 'a026_wb_advert_daily'
  AND turnover_code = 'advertising_allocated';
