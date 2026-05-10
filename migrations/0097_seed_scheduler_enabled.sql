-- Seed default value for the scheduler_enabled setting.
-- Migration 0096 used CREATE TABLE IF NOT EXISTS (no-op, table existed in baseline) and
-- INSERT OR IGNORE without created_at, which was silently skipped due to NOT NULL constraint.
-- This migration inserts the row correctly if it does not exist yet.
INSERT OR IGNORE INTO sys_settings (key, value, description, created_at, updated_at)
VALUES (
    'scheduler_enabled',
    'true',
    'Global scheduler on/off switch. Set to "false" to pause all scheduled task execution.',
    datetime('now'),
    datetime('now')
);
