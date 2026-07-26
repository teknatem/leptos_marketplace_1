-- Three-level LLM skill policy by virtual employee specialization.

CREATE TABLE IF NOT EXISTS sys_llm_skill_access (
    specialization TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    access_level TEXT NOT NULL CHECK (access_level IN ('immediate', 'extended', 'denied')),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (specialization, skill_id)
);

CREATE TABLE IF NOT EXISTS sys_llm_specialization_capability (
    specialization TEXT NOT NULL,
    capability TEXT NOT NULL,
    is_allowed INTEGER NOT NULL DEFAULT 0 CHECK (is_allowed IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (specialization, capability)
);

ALTER TABLE a018_llm_chat_message ADD COLUMN skill_trace_json TEXT;

-- Rename the legacy coordinator specialization while keeping application-level
-- read compatibility for old databases, imports and historical payloads.
UPDATE a017_llm_agent SET agent_type = 'coordinator_admin' WHERE agent_type = 'general';
UPDATE a038_llm_connection SET agent_type = 'coordinator_admin' WHERE agent_type = 'general';
UPDATE a039_mail_message SET agent_type = 'coordinator_admin' WHERE agent_type = 'general';

-- Existing default skills become immediate.
INSERT OR IGNORE INTO sys_llm_skill_access (specialization, skill_id, access_level) VALUES
    ('business_analyst', 'data-analytics', 'immediate'),
    ('system_admin', 'system-admin', 'immediate'),
    ('kb_admin', 'kb-curation', 'immediate'),
    ('plugin_admin', 'plugin-authoring', 'immediate'),
    ('sales_analyst', 'sales-analytics', 'immediate'),
    ('marketer', 'marketing-analytics', 'immediate'),
    ('financier', 'finance-analytics', 'immediate');

-- Existing optional skills become extended.
INSERT OR IGNORE INTO sys_llm_skill_access (specialization, skill_id, access_level) VALUES
    ('business_analyst', 'bi-authoring', 'extended'),
    ('business_analyst', 'chart-builder', 'extended'),
    ('business_analyst', 'table-builder', 'extended'),
    ('business_analyst', 'mailbox', 'extended'),
    ('plugin_admin', 'data-analytics', 'extended'),
    ('plugin_admin', 'chart-builder', 'extended'),
    ('plugin_admin', 'table-builder', 'extended'),
    ('sales_analyst', 'data-analytics', 'extended'),
    ('sales_analyst', 'bi-authoring', 'extended'),
    ('sales_analyst', 'chart-builder', 'extended'),
    ('sales_analyst', 'table-builder', 'extended'),
    ('sales_analyst', 'price-optimization', 'extended'),
    ('marketer', 'data-analytics', 'extended'),
    ('marketer', 'sales-analytics', 'extended'),
    ('marketer', 'chart-builder', 'extended'),
    ('marketer', 'table-builder', 'extended'),
    ('financier', 'data-analytics', 'extended'),
    ('financier', 'bi-authoring', 'extended'),
    ('financier', 'chart-builder', 'extended'),
    ('financier', 'table-builder', 'extended'),
    ('financier', 'price-optimization', 'extended');

-- CoordinatorAdmin receives immediate access to the current catalog. Skills
-- added later inherit the same effective default without a seed row.
INSERT OR IGNORE INTO sys_llm_skill_access (specialization, skill_id, access_level)
SELECT 'coordinator_admin', skill_id, 'immediate'
FROM (
    SELECT 'data-analytics' AS skill_id UNION ALL
    SELECT 'bi-authoring' UNION ALL
    SELECT 'plugin-authoring' UNION ALL
    SELECT 'chart-builder' UNION ALL
    SELECT 'table-builder' UNION ALL
    SELECT 'system-admin' UNION ALL
    SELECT 'mailbox' UNION ALL
    SELECT 'kb-curation' UNION ALL
    SELECT 'marketing-analytics' UNION ALL
    SELECT 'sales-analytics' UNION ALL
    SELECT 'finance-analytics' UNION ALL
    SELECT 'price-optimization'
);

-- Preserve the effective ability to publish artifacts for existing builder roles.
INSERT OR IGNORE INTO sys_llm_specialization_capability
    (specialization, capability, is_allowed)
VALUES
    ('coordinator_admin', 'artifact_publish', 1),
    ('business_analyst', 'artifact_publish', 1),
    ('plugin_admin', 'artifact_publish', 1),
    ('sales_analyst', 'artifact_publish', 1),
    ('marketer', 'artifact_publish', 1),
    ('financier', 'artifact_publish', 1),
    ('system_admin', 'artifact_publish', 0),
    ('kb_admin', 'artifact_publish', 0);
