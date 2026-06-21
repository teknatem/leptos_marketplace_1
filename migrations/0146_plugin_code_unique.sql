-- Enforce portable plugin identity at the database boundary.
-- Keep the newest non-deleted row for each code before creating the unique index.

WITH ranked AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            PARTITION BY code
            ORDER BY datetime(updated_at) DESC, datetime(created_at) DESC, id DESC
        ) AS rn
    FROM plugin
    WHERE is_deleted = 0
)
UPDATE plugin
SET
    is_deleted = 1,
    updated_at = datetime('now')
WHERE id IN (SELECT id FROM ranked WHERE rn > 1);

DROP INDEX IF EXISTS idx_plugin_code;

CREATE UNIQUE INDEX IF NOT EXISTS ux_plugin_code_active
    ON plugin (code)
    WHERE is_deleted = 0;
