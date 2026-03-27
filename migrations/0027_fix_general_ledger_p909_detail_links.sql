-- Historical p909 rows were linked to general ledger correctly through
-- p909.general_ledger_ref, but sys_general_ledger.detail_id still kept the
-- general ledger id instead of the analytical row id. This migration restores
-- the canonical navigation key: general_ledger.detail_id -> p909.id.

UPDATE sys_general_ledger
SET detail_id = (
    SELECT p.id
    FROM p909_mp_order_line_turnovers p
    WHERE p.general_ledger_ref = sys_general_ledger.id
    LIMIT 1
)
WHERE detail_kind = 'p909_mp_order_line_turnovers'
  AND EXISTS (
      SELECT 1
      FROM p909_mp_order_line_turnovers p
      WHERE p.general_ledger_ref = sys_general_ledger.id
  );
