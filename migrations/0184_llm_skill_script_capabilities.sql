-- Runtime capabilities for executable tasks bundled with LLM skill packages.
--
-- Existing installations have no executable skill tasks, so enabling these
-- capabilities for CoordinatorAdmin and PluginAdmin does not broaden access
-- to current tools. Other specializations remain denied until configured in
-- the access matrix.

INSERT OR IGNORE INTO sys_llm_specialization_capability
    (specialization, capability, is_allowed, updated_at)
VALUES
    ('coordinator_admin', 'skill_script_execute', 1, CURRENT_TIMESTAMP),
    ('coordinator_admin', 'skill_script_develop', 1, CURRENT_TIMESTAMP),
    ('plugin_admin',      'skill_script_execute', 1, CURRENT_TIMESTAMP),
    ('plugin_admin',      'skill_script_develop', 1, CURRENT_TIMESTAMP)
;

INSERT OR IGNORE INTO sys_llm_specialization_capability
    (specialization, capability, is_allowed, updated_at)
SELECT specialization, capability, 0, CURRENT_TIMESTAMP
FROM (
    SELECT 'business_analyst' AS specialization UNION ALL
    SELECT 'sales_analyst' UNION ALL
    SELECT 'marketer' UNION ALL
    SELECT 'financier' UNION ALL
    SELECT 'system_admin' UNION ALL
    SELECT 'kb_admin'
) AS specializations
CROSS JOIN (
    SELECT 'skill_script_execute' AS capability UNION ALL
    SELECT 'skill_script_develop'
) AS capabilities;
