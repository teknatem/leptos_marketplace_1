-- Migration 0131: FINA-layer revenue BI indicators + add them to dashboard `test`.
--
-- Three ready-to-use BI indicators on DataView dv004_general_ledger_turnovers,
-- reading sys_general_ledger at layer = 'fina':
--   1. Продажи (фин)  -> turnover_code  = customer_revenue
--   2. Возвраты (фин) -> turnover_code  = customer_revenue_storno
--   3. Выручка (фин)  -> turnover_items = customer_revenue, customer_revenue_storno  (sum)
--
-- The composite #3 uses comma-separated turnover_items (NOT semicolons) so the
-- migration runner, which splits on ';', does not break the statement.
--
-- Idempotent: INSERT OR IGNORE on stable ids; dashboard group is appended only
-- when not already present (guard on '"Финансы"'), and is a no-op if no dashboard
-- with code='test' exists.

-- ---------------------------------------------------------------------------
-- 1) Продажи (фин)
-- ---------------------------------------------------------------------------
INSERT OR IGNORE INTO a024_bi_indicator (
    id, code, description, comment,
    data_spec_json, params_json, view_spec_json, drill_spec_json,
    status, owner_user_id, is_public, created_by, updated_by,
    is_deleted, is_posted, created_at, updated_at, version
) VALUES (
    'a024a024-0131-4001-a001-000000000131',
    'IND-FINA-SALES',
    'Продажи (фин)',
    'Оборот customer_revenue по слою FINA за выбранный период и кабинеты.',
    '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"customer_revenue","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"fina","required":true,"global_filter_key":null}]',
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
);

-- ---------------------------------------------------------------------------
-- 2) Возвраты (фин)
-- ---------------------------------------------------------------------------
INSERT OR IGNORE INTO a024_bi_indicator (
    id, code, description, comment,
    data_spec_json, params_json, view_spec_json, drill_spec_json,
    status, owner_user_id, is_public, created_by, updated_by,
    is_deleted, is_posted, created_at, updated_at, version
) VALUES (
    'a024a024-0132-4001-a001-000000000132',
    'IND-FINA-RETURNS',
    'Возвраты (фин)',
    'Оборот customer_revenue_storno по слою FINA (хранится отрицательной суммой) за выбранный период.',
    '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    '[{"key":"turnover_code","param_type":"string","label":"Turnover code","default_value":"customer_revenue_storno","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"fina","required":true,"global_filter_key":null}]',
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
);

-- ---------------------------------------------------------------------------
-- 3) Выручка (фин) = customer_revenue + customer_revenue_storno
-- ---------------------------------------------------------------------------
INSERT OR IGNORE INTO a024_bi_indicator (
    id, code, description, comment,
    data_spec_json, params_json, view_spec_json, drill_spec_json,
    status, owner_user_id, is_public, created_by, updated_by,
    is_deleted, is_posted, created_at, updated_at, version
) VALUES (
    'a024a024-0133-4001-a001-000000000133',
    'IND-FINA-REVENUE',
    'Выручка (фин)',
    'Сумма оборотов customer_revenue + customer_revenue_storno по слою FINA за выбранный период.',
    '{"view_id":"dv004_general_ledger_turnovers","metric_id":"amount"}',
    '[{"key":"turnover_items","param_type":"string","label":"Turnover items","default_value":"customer_revenue, customer_revenue_storno","required":true,"global_filter_key":null},{"key":"layer","param_type":"string","label":"Layer","default_value":"fina","required":true,"global_filter_key":null}]',
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
);

-- ---------------------------------------------------------------------------
-- Add a "Финансы" group with the 3 indicators to dashboard `test`.
-- Appends to layout_json.$.groups only if the group is not already there.
-- ---------------------------------------------------------------------------
UPDATE a025_bi_dashboard
SET layout_json = json_insert(
        layout_json,
        '$.groups[#]',
        json('{"id":"01310131-0000-4001-a001-000000000131","title":"Финансы","sort_order":100,"items":[{"indicator_id":"a024a024-0131-4001-a001-000000000131","indicator_name":"IND-FINA-SALES — Продажи (фин)","sort_order":0,"col_class":"1x1","param_overrides":{}},{"indicator_id":"a024a024-0132-4001-a001-000000000132","indicator_name":"IND-FINA-RETURNS — Возвраты (фин)","sort_order":1,"col_class":"1x1","param_overrides":{}},{"indicator_id":"a024a024-0133-4001-a001-000000000133","indicator_name":"IND-FINA-REVENUE — Выручка (фин)","sort_order":2,"col_class":"1x1","param_overrides":{}}],"subgroups":[]}')
    ),
    updated_at = datetime('now'),
    version = version + 1
WHERE (code = 'test' OR description = 'test')
  AND layout_json NOT LIKE '%"Финансы"%';
