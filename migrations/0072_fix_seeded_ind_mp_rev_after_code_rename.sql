-- Correct the seeded IND-MP-REV record even if its code was renamed manually.
-- Uses the stable seeded UUID instead of code to avoid missing the row.

UPDATE a024_bi_indicator
SET
    data_spec_json = '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    params_json = '[{"key":"turnover_items","param_type":"string","label":"GL turnovers","default_value":"customer_revenue, customer_return","required":true,"global_filter_key":null},{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"customer_revenue","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"fact","required":true,"global_filter_key":null}]',
    comment = 'GL-first indicator on dv004_general_ledger_turnovers. Net revenue is computed as customer_revenue + customer_return on layer=fact.',
    updated_by = 'system',
    updated_at = datetime('now'),
    version = COALESCE(version, 0) + 1
WHERE id = 'a024a024-0010-4001-a001-000000000010';
