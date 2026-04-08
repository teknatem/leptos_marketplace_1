-- Switch IND-MP-RETURNS from dv003 oper metric to GL turnover-based dv004.

UPDATE a024_bi_indicator
SET
    description = 'Возвраты MP',
    comment = 'Готовый BI-индикатор на DataView dv004_general_ledger_turnovers, turnover_code=customer_return, layer=fact',
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"customer_return","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"fact","required":true,"global_filter_key":null}]',
    updated_by = 'system',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-RETURNS';
