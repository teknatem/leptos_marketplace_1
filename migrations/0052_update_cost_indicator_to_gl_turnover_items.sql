UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_items","param_type":"string","label":"GL turnovers","default_value":"item_cost, item_cost_storno","required":true,"global_filter_key":null},{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"item_cost","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Migrated to dv004_general_ledger_turnovers. COST is computed as item_cost + item_cost_storno on layer=oper via turnover_items.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'COST';
