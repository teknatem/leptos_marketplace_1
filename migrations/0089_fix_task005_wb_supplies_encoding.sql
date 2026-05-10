-- Fix display text for the seeded WB supplies scheduled task.
-- Metadata in the task type registry is provided by Rust code; this keeps
-- already-created sys_tasks rows readable on installations where the seed ran.

UPDATE sys_tasks
SET
    description = 'WB Поставки FBS — список + стикеры (каждый час). Замените connection_id на UUID WB-кабинета.',
    updated_at = datetime('now')
WHERE id = 'a1b2c3d4-e5f6-7890-abcd-ef1234567805'
  AND code = 'task005-wb-supplies'
  AND task_type = 'task005_wb_supplies'
  AND is_deleted = 0;
