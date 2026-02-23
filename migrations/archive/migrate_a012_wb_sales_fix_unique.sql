-- Migration: Fix UNIQUE constraint on a012_wb_sales
-- Problem: Table has UNIQUE(document_no) but needs UNIQUE(document_no, event_type, supplier_article)
-- Reason: Same SRID (document_no) can have multiple entries for different event types or articles
-- Date: 2026-01-21

-- Step 1: Create new table with correct schema
CREATE TABLE a012_wb_sales_new (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    comment TEXT,
    document_no TEXT NOT NULL,
    sale_id TEXT,
    -- Denormalized fields from JSON for fast queries
    sale_date TEXT,
    organization_id TEXT,
    connection_id TEXT,
    supplier_article TEXT,
    nm_id INTEGER,
    barcode TEXT,
    product_name TEXT,
    qty REAL,
    amount_line REAL,
    total_price REAL,
    finished_price REAL,
    event_type TEXT,
    -- JSON storage (kept for backward compatibility and full data)
    header_json TEXT NOT NULL,
    line_json TEXT NOT NULL,
    state_json TEXT NOT NULL,
    warehouse_json TEXT,
    source_meta_json TEXT NOT NULL,
    marketplace_product_ref TEXT,
    nomenclature_ref TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 0,
    -- Composite unique constraint: same SRID can exist for different event_types/articles
    UNIQUE (document_no, event_type, supplier_article)
);

-- Step 2: Copy data from old table (handling duplicates)
-- Keep only the latest version of each unique combination
INSERT INTO a012_wb_sales_new
SELECT * FROM a012_wb_sales
WHERE id IN (
    SELECT id FROM (
        SELECT id, 
               ROW_NUMBER() OVER (
                   PARTITION BY document_no, event_type, supplier_article 
                   ORDER BY updated_at DESC, created_at DESC
               ) as rn
        FROM a012_wb_sales
    )
    WHERE rn = 1
);

-- Step 3: Drop old table
DROP TABLE a012_wb_sales;

-- Step 4: Rename new table
ALTER TABLE a012_wb_sales_new RENAME TO a012_wb_sales;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_a012_sale_date ON a012_wb_sales(sale_date);
CREATE INDEX IF NOT EXISTS idx_a012_organization ON a012_wb_sales(organization_id);
CREATE INDEX IF NOT EXISTS idx_a012_sale_id ON a012_wb_sales(sale_id);
CREATE INDEX IF NOT EXISTS idx_a012_document_no ON a012_wb_sales(document_no);

-- Verification queries (run manually after migration):
-- SELECT COUNT(*) as total FROM a012_wb_sales;
-- SELECT document_no, event_type, supplier_article, COUNT(*) as cnt 
-- FROM a012_wb_sales 
-- GROUP BY document_no, event_type, supplier_article 
-- HAVING cnt > 1;
-- Expected: 0 rows (no duplicates)

-- Diagnostic: Show any potential duplicates before migration
-- SELECT document_no, event_type, supplier_article, COUNT(*) as duplicates
-- FROM a012_wb_sales
-- GROUP BY document_no, event_type, supplier_article
-- HAVING COUNT(*) > 1;
