ALTER TABLE sys_general_ledger
ADD COLUMN cabinet_mp TEXT;

UPDATE sys_general_ledger
SET cabinet_mp = (
    SELECT p909.connection_mp_ref
    FROM p909_mp_order_line_turnovers p909
    WHERE p909.general_ledger_ref = sys_general_ledger.id
    LIMIT 1
)
WHERE cabinet_mp IS NULL;

UPDATE sys_general_ledger
SET cabinet_mp = (
    SELECT p910.connection_mp_ref
    FROM p910_mp_unlinked_turnovers p910
    WHERE p910.general_ledger_ref = sys_general_ledger.id
    LIMIT 1
)
WHERE cabinet_mp IS NULL;

UPDATE sys_general_ledger
SET cabinet_mp = (
    SELECT p903.connection_mp_ref
    FROM p903_wb_finance_report p903
    WHERE p903.id = sys_general_ledger.registrator_ref
    LIMIT 1
)
WHERE cabinet_mp IS NULL
  AND registrator_type = 'p903_wb_finance_report';

UPDATE sys_general_ledger
SET cabinet_mp = (
    SELECT a026.connection_id
    FROM a026_wb_advert_daily a026
    WHERE ('a026:' || a026.id) = sys_general_ledger.registrator_ref
    LIMIT 1
)
WHERE cabinet_mp IS NULL
  AND registrator_type = 'a026_wb_advert_daily';

CREATE INDEX IF NOT EXISTS idx_sgl_cabinet_mp
    ON sys_general_ledger (cabinet_mp);
