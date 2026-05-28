-- 0125 p907_ym_payment_report: normalise id column to standard UUID v4 format
-- Migration 0124 filled existing rows with lower(hex(randomblob(16))) which produces
-- 32-char hex without dashes. All other domain tables use hyphenated UUID v4 (36 chars).
-- This migration re-generates all p907 ids as proper UUID v4 strings so the format
-- matches the rest of the project.
--
-- p907 is a pure projection (re-importable from YM), so regenerating ids is safe.
-- The unique index is dropped before the bulk update and re-created after.

DROP INDEX IF EXISTS idx_p907_id;

UPDATE p907_ym_payment_report
    SET id = lower(
        substr(hex(randomblob(4)),  1, 8) || '-' ||
        substr(hex(randomblob(2)),  1, 4) || '-' ||
        '4' || substr(hex(randomblob(2)), 2, 3) || '-' ||
        substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2, 3) || '-' ||
        substr(hex(randomblob(6)),  1, 12)
    );

CREATE UNIQUE INDEX IF NOT EXISTS idx_p907_id ON p907_ym_payment_report (id);
