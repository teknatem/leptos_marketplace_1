-- Seed: атомарные WB-задания (task003–task011)
-- ВНИМАНИЕ: Перед включением замените connection_id на реальный UUID WB-кабинета
--           из справочника a006_connection_mp. Задания создаются отключёнными (is_enabled = 0).

-- task003: Товары (каталог, раз в день)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567803',
    'task003-wb-products',
    'WB Товары — синхронизация каталога (раз в день). Замените connection_id на UUID WB-кабинета.',
    'task003_wb_products',
    '0 0 3 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID"}',
    0, datetime('now'), datetime('now'), 0
);

-- task004: Продажи (Statistics API, раз в день)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567804',
    'task004-wb-sales',
    'WB Продажи — Statistics API (раз в день). Замените connection_id на UUID WB-кабинета.',
    'task004_wb_sales',
    '0 0 4 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","lookback_days":30,"overlap_days":1}',
    0, datetime('now'), datetime('now'), 0
);

-- task005: Поставки FBS (каждый час)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567805',
    'task005-wb-supplies',
    'WB Поставки FBS — список + стикеры (каждый час). Замените connection_id на UUID WB-кабинета.',
    'task005_wb_supplies',
    '0 0 * * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","lookback_days":7}',
    0, datetime('now'), datetime('now'), 0
);

-- task006: Финансовый отчёт (раз в день, ночью — долго из-за 1 req/min)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567806',
    'task006-wb-finance',
    'WB Финансовый отчёт — Statistics API, 1 req/min (раз в день). Замените connection_id на UUID WB-кабинета.',
    'task006_wb_finance',
    '0 0 1 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","lookback_days":14,"overlap_days":3}',
    0, datetime('now'), datetime('now'), 0
);

-- task007: Тарифы и комиссии (раз в неделю)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567807',
    'task007-wb-commissions',
    'WB Тарифы и комиссии — снимок (раз в неделю). Замените connection_id на UUID WB-кабинета.',
    'task007_wb_commissions',
    '0 0 2 * * 1',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID"}',
    0, datetime('now'), datetime('now'), 0
);

-- task008: Цены и скидки (2 раза в день)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567808',
    'task008-wb-prices',
    'WB Цены и скидки — текущий срез (2 раза в день). Замените connection_id на UUID WB-кабинета.',
    'task008_wb_prices',
    '0 0 6,18 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID"}',
    0, datetime('now'), datetime('now'), 0
);

-- task009: Промоакции (раз в день)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567809',
    'task009-wb-promotions',
    'WB Промоакции — Calendar API (раз в день). Замените connection_id на UUID WB-кабинета.',
    'task009_wb_promotions',
    '0 0 5 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","lookback_days":30,"overlap_days":1}',
    0, datetime('now'), datetime('now'), 0
);

-- task010: Документы (раз в день)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567810',
    'task010-wb-documents',
    'WB Документы — Documents API (раз в день). Замените connection_id на UUID WB-кабинета.',
    'task010_wb_documents',
    '0 0 5 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","lookback_days":30,"overlap_days":1}',
    0, datetime('now'), datetime('now'), 0
);

-- task011: Реклама (раз в день)
INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567811',
    'task011-wb-advert',
    'WB Реклама — статистика кампаний (раз в день). Замените connection_id на UUID WB-кабинета.',
    'task011_wb_advert',
    '0 0 6 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID","lookback_days":7,"overlap_days":1}',
    0, datetime('now'), datetime('now'), 0
);
