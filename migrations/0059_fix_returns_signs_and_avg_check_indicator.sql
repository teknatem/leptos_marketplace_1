UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_items","param_type":"string","label":"GL turnovers","default_value":"-customer_return","required":true,"global_filter_key":null},{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"customer_return","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"fact","required":true,"global_filter_key":null}]',
    comment = 'Сумма возвратов маркетплейса по фактическим GL-оборотам за выбранный период, со знаком плюс для отображения.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-RETURNS';

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv007_gl_turnover_ratio_percent","metric_id":"ratio_percent"}',
    params_json = '[{"key":"numerator_turnover_items","param_type":"string","label":"Numerator GL turnovers","default_value":"-customer_revenue_pl_storno","required":true,"global_filter_key":null},{"key":"numerator_turnover_code","param_type":"string","label":"Numerator turnover code","default_value":"customer_revenue_pl_storno","required":true,"global_filter_key":null},{"key":"numerator_layer","param_type":"string","label":"Numerator layer","default_value":"oper","required":true,"global_filter_key":null},{"key":"denominator_turnover_code","param_type":"string","label":"Denominator turnover code","default_value":"customer_revenue_pl","required":true,"global_filter_key":null},{"key":"denominator_layer","param_type":"string","label":"Denominator layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Доля возвратов в реализации: сумма customer_revenue_pl_storno к customer_revenue_pl, в процентах, со знаком плюс для отображения.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-RETURNS-TO-REV-PCT';

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv006_indicator_ratio_percent","metric_id":"ratio"}',
    params_json = '[{"key":"numerator_indicator_code","param_type":"string","label":"Numerator indicator code","default_value":"IND-MP-REV-PRICE","required":true,"global_filter_key":null},{"key":"denominator_indicator_code","param_type":"string","label":"Denominator indicator code","default_value":"IND-ORDERS","required":true,"global_filter_key":null}]',
    comment = 'Средний чек как отношение полной выручки IND-MP-REV-PRICE к количеству заказов IND-ORDERS.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-AVG-CHECK';
