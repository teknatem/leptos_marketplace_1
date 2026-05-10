-- Explicit data watermark for windowed scheduled imports.
-- Stored as YYYY-MM-DD and means: data is loaded inclusively up to this date.
ALTER TABLE sys_tasks ADD COLUMN data_loaded_up_to TEXT;

-- Preserve existing behavior for already configured tasks: the old field was
-- used as a data watermark, so copy its date part into the explicit field.
UPDATE sys_tasks
SET data_loaded_up_to = substr(last_successful_run_at, 1, 10)
WHERE last_successful_run_at IS NOT NULL
  AND data_loaded_up_to IS NULL;
