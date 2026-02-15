-- Migration: Create a015_wb_orders table for Wildberries Orders
-- This table stores order documents from Wildberries API
-- Date: 2026-02-11

CREATE TABLE IF NOT EXISTS a015_wb_orders (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    comment TEXT,
    document_no TEXT NOT NULL,
    document_date TEXT,
    g_number TEXT,
    spp REAL,
    is_cancel INTEGER,
    cancel_date TEXT,
    header_json TEXT NOT NULL,
    line_json TEXT NOT NULL,
    state_json TEXT NOT NULL,
    warehouse_json TEXT NOT NULL,
    geography_json TEXT NOT NULL,
    source_meta_json TEXT NOT NULL,
    marketplace_product_ref TEXT,
    nomenclature_ref TEXT,
    base_nomenclature_ref TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 0
);

-- Indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_a015_document_no ON a015_wb_orders(document_no);
CREATE INDEX IF NOT EXISTS idx_a015_g_number ON a015_wb_orders(g_number);
CREATE INDEX IF NOT EXISTS idx_a015_is_posted ON a015_wb_orders(is_posted);
CREATE INDEX IF NOT EXISTS idx_a015_is_cancel ON a015_wb_orders(is_cancel);

-- Add column for existing databases (run manually only if column is missing):
-- ALTER TABLE a015_wb_orders ADD COLUMN base_nomenclature_ref TEXT;

-- Verification queries (run manually after migration):

-- Check that table was created:
-- SELECT name FROM sqlite_master WHERE type='table' AND name='a015_wb_orders';

-- Check column count:
-- SELECT COUNT(*) as column_count FROM pragma_table_info('a015_wb_orders');

-- Check table structure:
-- PRAGMA table_info(a015_wb_orders);

-- Sample data check:
-- SELECT id, document_no, g_number, is_cancel, is_posted, created_at
-- FROM a015_wb_orders 
-- LIMIT 5;

-- Count records:
-- SELECT COUNT(*) as total FROM a015_wb_orders;
