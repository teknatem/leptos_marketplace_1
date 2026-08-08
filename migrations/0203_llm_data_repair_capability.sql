INSERT OR REPLACE INTO sys_llm_specialization_capability
    (specialization, capability, is_allowed, updated_at)
SELECT specialization, 'data_repair_execute',
       CASE WHEN specialization = 'coordinator_admin' THEN 1 ELSE 0 END,
       CURRENT_TIMESTAMP
FROM (
    SELECT 'business_analyst' AS specialization UNION ALL
    SELECT 'sales_analyst' UNION ALL
    SELECT 'marketer' UNION ALL
    SELECT 'financier' UNION ALL
    SELECT 'system_admin' UNION ALL
    SELECT 'kb_admin' UNION ALL
    SELECT 'plugin_admin' UNION ALL
    SELECT 'coordinator_admin' UNION ALL
    SELECT 'tester'
);
