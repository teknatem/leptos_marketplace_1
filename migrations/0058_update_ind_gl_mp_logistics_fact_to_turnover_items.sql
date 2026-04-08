UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_items","param_type":"string","label":"GL turnovers","default_value":"mp_ppvz_reward, mp_ppvz_reward_nm, mp_rebill_logistic_cost, mp_rebill_logistic_cost_nm","required":true,"global_filter_key":null},{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"mp_ppvz_reward","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"fact","required":true,"global_filter_key":null}]',
    comment = 'Ready-to-use BI indicator on DataView dv004_general_ledger_turnovers with turnover_items=mp_ppvz_reward, mp_ppvz_reward_nm, mp_rebill_logistic_cost, mp_rebill_logistic_cost_nm and layer=fact.',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-GL-MP-LOGISTICS-FACT';
