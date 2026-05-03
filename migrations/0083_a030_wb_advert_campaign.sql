CREATE TABLE IF NOT EXISTS a030_wb_advert_campaign (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL,
    description TEXT NOT NULL,
    comment TEXT,
    advert_id INTEGER NOT NULL,
    connection_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    marketplace_id TEXT NOT NULL,
    campaign_type INTEGER,
    status INTEGER,
    change_time TEXT,
    info_json TEXT NOT NULL DEFAULT '{}',
    source_meta_json TEXT NOT NULL DEFAULT '{}',
    is_deleted BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a030_wb_advert_campaign_connection_advert
    ON a030_wb_advert_campaign(connection_id, advert_id)
    WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_a030_wb_advert_campaign_connection
    ON a030_wb_advert_campaign(connection_id);

CREATE INDEX IF NOT EXISTS idx_a030_wb_advert_campaign_status
    ON a030_wb_advert_campaign(status);

INSERT OR IGNORE INTO sys_tasks (id, code, description, task_type, schedule_cron, config_json, is_enabled, created_at, updated_at, is_deleted)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567812',
    'task012-wb-advert-campaigns',
    'WB Реклама — справочник кампаний (4 раза в день). Замените connection_id на UUID WB-кабинета.',
    'task012_wb_advert_campaigns',
    '0 15 0,6,12,18 * * *',
    '{"connection_id":"REPLACE_WITH_WB_CONNECTION_ID"}',
    0, datetime('now'), datetime('now'), 0
);
