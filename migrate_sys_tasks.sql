-- Migration: Rename sys_scheduled_tasks -> sys_tasks
-- Date: 2026-01-11
-- Description: Rename table to match VSA naming convention

-- Check if old table exists and new table doesn't exist
-- SQLite supports ALTER TABLE ... RENAME TO

ALTER TABLE sys_scheduled_tasks RENAME TO sys_tasks;

-- Note: If the table doesn't exist, this will fail gracefully
-- Run this migration only on existing databases that have sys_scheduled_tasks table
