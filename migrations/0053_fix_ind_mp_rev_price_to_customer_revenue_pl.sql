-- Correct IND-MP-REV-PRICE GL mapping: price-list revenue must use customer_revenue_pl.

UPDATE a024_bi_indicator
SET
    description = 'Выручка (пл) MP',
    comment = 'Ready-to-use BI indicator on DataView dv004_general_ledger_turnovers with turnover_code=customer_revenue_pl and layer=oper.',
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"customer_revenue_pl","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    updated_by = 'system',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-REV-PRICE';
