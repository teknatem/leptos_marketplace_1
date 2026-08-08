-- Quality results are useful to every specialization. Executable rule
-- authoring remains a narrow plugin_admin responsibility.
INSERT OR REPLACE INTO sys_llm_skill_access (specialization, skill_id, access_level)
SELECT specialization, 'quality-monitoring', 'immediate'
FROM (
    SELECT 'business_analyst' AS specialization UNION ALL
    SELECT 'sales_analyst' UNION ALL
    SELECT 'marketer' UNION ALL
    SELECT 'financier' UNION ALL
    SELECT 'system_admin' UNION ALL
    SELECT 'kb_admin' UNION ALL
    SELECT 'plugin_admin' UNION ALL
    SELECT 'coordinator_admin'
);

INSERT OR REPLACE INTO sys_llm_skill_access (specialization, skill_id, access_level)
VALUES ('plugin_admin', 'quality-check-authoring', 'immediate');

INSERT OR REPLACE INTO sys_llm_skill_access (specialization, skill_id, access_level)
SELECT specialization, 'quality-check-authoring', 'denied'
FROM (
    SELECT 'business_analyst' AS specialization UNION ALL
    SELECT 'sales_analyst' UNION ALL
    SELECT 'marketer' UNION ALL
    SELECT 'financier' UNION ALL
    SELECT 'system_admin' UNION ALL
    SELECT 'kb_admin' UNION ALL
    SELECT 'coordinator_admin'
);
