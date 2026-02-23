-- Migration: Update dashboard configs to use new schema ID
-- Date: 2026-02-04
-- Description: Change data_source from legacy 'p903_wb_finance_report' to 's001_wb_finance'
--              This ensures configs work with the unified schema registry

-- Update all configs that use the old P903 schema ID
UPDATE sys_dashboard_configs 
SET data_source = 's001_wb_finance' 
WHERE data_source = 'p903_wb_finance_report';

-- Verify the update
SELECT 
    data_source,
    COUNT(*) as config_count,
    GROUP_CONCAT(name, ', ') as config_names
FROM sys_dashboard_configs
WHERE data_source LIKE '%finance%'
GROUP BY data_source;
