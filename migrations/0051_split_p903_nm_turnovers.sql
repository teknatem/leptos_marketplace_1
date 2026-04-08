WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE sys_general_ledger
SET turnover_code = 'mp_commission_adjustment_nm'
WHERE turnover_code = 'mp_commission_adjustment'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE sys_general_ledger
SET turnover_code = 'mp_rebill_logistic_cost_nm'
WHERE turnover_code = 'mp_rebill_logistic_cost'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE sys_general_ledger
SET turnover_code = 'mp_ppvz_reward_nm'
WHERE turnover_code = 'mp_ppvz_reward'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE p909_mp_order_line_turnovers
SET turnover_code = 'mp_rebill_logistic_cost_nm'
WHERE turnover_code = 'mp_rebill_logistic_cost'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE p909_mp_order_line_turnovers
SET turnover_code = 'mp_commission_adjustment_nm'
WHERE turnover_code = 'mp_commission_adjustment'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE p909_mp_order_line_turnovers
SET turnover_code = 'mp_ppvz_reward_nm'
WHERE turnover_code = 'mp_ppvz_reward'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE p910_mp_unlinked_turnovers
SET turnover_code = 'mp_rebill_logistic_cost_nm'
WHERE turnover_code = 'mp_rebill_logistic_cost'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE p910_mp_unlinked_turnovers
SET turnover_code = 'mp_commission_adjustment_nm'
WHERE turnover_code = 'mp_commission_adjustment'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH p903_with_nm AS (
    SELECT id
    FROM p903_wb_finance_report
    WHERE nm_id IS NOT NULL
      AND nm_id != 0
)
UPDATE p910_mp_unlinked_turnovers
SET turnover_code = 'mp_ppvz_reward_nm'
WHERE turnover_code = 'mp_ppvz_reward'
  AND registrator_type = 'p903_wb_finance_report'
  AND registrator_ref IN (SELECT id FROM p903_with_nm);

WITH grouped AS (
    SELECT
        connection_mp_ref,
        line_event_key,
        turnover_code,
        MAX(CASE WHEN layer = 'plan' THEN 1 ELSE 0 END) AS has_plan,
        MAX(CASE WHEN layer = 'oper' THEN 1 ELSE 0 END) AS has_oper,
        MAX(CASE WHEN layer = 'fact' THEN 1 ELSE 0 END) AS has_fact
    FROM p909_mp_order_line_turnovers
    GROUP BY connection_mp_ref, line_event_key, turnover_code
)
UPDATE p909_mp_order_line_turnovers
SET link_status = (
    SELECT CASE
        WHEN grouped.has_plan = 1 AND grouped.has_oper = 1 AND grouped.has_fact = 1 THEN 'full'
        WHEN grouped.has_plan = 1 AND grouped.has_oper = 1 AND grouped.has_fact = 0 THEN 'oper_plan'
        WHEN grouped.has_plan = 1 AND grouped.has_oper = 0 AND grouped.has_fact = 1 THEN 'fact_plan'
        WHEN grouped.has_plan = 0 AND grouped.has_oper = 1 AND grouped.has_fact = 1 THEN 'oper_fact'
        ELSE 'single'
    END
    FROM grouped
    WHERE grouped.connection_mp_ref = p909_mp_order_line_turnovers.connection_mp_ref
      AND grouped.line_event_key = p909_mp_order_line_turnovers.line_event_key
      AND grouped.turnover_code = p909_mp_order_line_turnovers.turnover_code
);
