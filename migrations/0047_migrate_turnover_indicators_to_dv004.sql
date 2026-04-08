-- Move direct turnover indicators from detail projections to GL-first dv004.
-- Formula-based and non-GL indicators are intentionally excluded.

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"customer_revenue","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Migrated to GL-first dv004_general_ledger_turnovers. Preserves historical dv003 metric=revenue_price semantics via turnover_code=customer_revenue, layer=oper.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-REV-PRICE';

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"wb_coinvestment","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Migrated to GL-first dv004_general_ledger_turnovers. Primary scalar comes from sys_general_ledger, drilldown uses GL drilldown. turnover_code=wb_coinvestment, layer=oper.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-COINVEST';

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"mp_acquiring","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Migrated to GL-first dv004_general_ledger_turnovers. Primary scalar comes from sys_general_ledger, drilldown uses GL drilldown. turnover_code=mp_acquiring, layer=oper.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-ACQUIRING';

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"item_cost","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Migrated to GL-first dv004_general_ledger_turnovers. Primary scalar comes from sys_general_ledger, drilldown uses GL drilldown. turnover_code=item_cost, layer=oper.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-COST';

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"mp_commission","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    comment = 'Migrated to GL-first dv004_general_ledger_turnovers. Primary scalar comes from sys_general_ledger, drilldown uses GL drilldown. turnover_code=mp_commission, layer=oper.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-COMMISSION';
