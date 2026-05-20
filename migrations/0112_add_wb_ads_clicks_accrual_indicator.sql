-- Ready-to-use BI indicator for total WB advertising click accrual.

INSERT INTO a024_bi_indicator (
    id,
    code,
    description,
    comment,
    explanation,
    data_spec_json,
    params_json,
    view_spec_json,
    drill_spec_json,
    status,
    owner_user_id,
    is_public,
    created_by,
    updated_by,
    is_deleted,
    is_posted,
    created_at,
    updated_at,
    version
)
SELECT
    'a024a024-0026-4001-a001-000000000026',
    'IND-WB-ADS-CLICKS-ACCRUAL',
    'Начисление за клики WB',
    'Суммарное начисление рекламных кликов WB из GL: advert_clicks_order_accrual + advert_clicks_no_order.',
    'DataView: dv004_general_ledger_turnovers, metric=amount. Формула: advert_clicks_order_accrual + advert_clicks_no_order, слой oper. Индикатор показывает полное начисление за рекламные клики: часть с аналитикой по заказам берётся из advert_clicks_order_accrual, часть без аналитики по заказам сразу в затраты берётся из advert_clicks_no_order. Оборот advert_clicks_order_expense не включается, потому что это второй этап списания уже начисленного резерва в расходы.',
    '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    '[{"key":"turnover_items","param_type":"string","label":"GL turnovers","default_value":"advert_clicks_order_accrual, advert_clicks_no_order","required":true,"global_filter_key":null},{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"advert_clicks_order_accrual","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"oper","required":true,"global_filter_key":null}]',
    '{"style_name":"classic","custom_html":null,"custom_css":null,"format":{"kind":"Money","currency":"RUB"},"thresholds":[],"preview_values":{}}',
    NULL,
    'active',
    'f2fc6986-855d-492b-acff-70c7cd8cdd34',
    1,
    'system',
    'system',
    0,
    1,
    datetime('now'),
    datetime('now'),
    1
WHERE NOT EXISTS (
    SELECT 1
    FROM a024_bi_indicator
    WHERE code = 'IND-WB-ADS-CLICKS-ACCRUAL'
);
