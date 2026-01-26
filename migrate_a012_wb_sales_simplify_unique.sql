-- Migration: Simplify UNIQUE constraint on a012_wb_sales
-- Change from: UNIQUE(document_no, event_type, supplier_article)
-- To: sale_id TEXT NOT NULL UNIQUE
-- Reason: Use only sale_id for deduplication (simpler and more reliable)
-- Date: 2026-01-21

-- Step 1: Create new table with correct schema (sale_id as UNIQUE key)
CREATE TABLE a012_wb_sales_new (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    comment TEXT,
    document_no TEXT NOT NULL,
    sale_id TEXT NOT NULL UNIQUE,
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
    version INTEGER NOT NULL DEFAULT 0
);

-- Step 2: Copy data from old table
-- Handle records without sale_id by generating it
INSERT INTO a012_wb_sales_new
SELECT 
    id,
    code,
    description,
    comment,
    document_no,
    COALESCE(
        sale_id, 
        'WB_GEN_' || document_no || '_' || COALESCE(event_type, 'unknown') || '_' || COALESCE(supplier_article, '')
    ) as sale_id,
    sale_date,
    organization_id,
    connection_id,
    supplier_article,
    nm_id,
    barcode,
    product_name,
    qty,
    amount_line,
    total_price,
    finished_price,
    event_type,
    header_json,
    line_json,
    state_json,
    warehouse_json,
    source_meta_json,
    marketplace_product_ref,
    nomenclature_ref,
    is_deleted,
    is_posted,
    created_at,
    updated_at,
    version
FROM a012_wb_sales
WHERE id IN (
    SELECT id FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY COALESCE(sale_id, 'WB_GEN_' || document_no || '_' || COALESCE(event_type, 'unknown') || '_' || COALESCE(supplier_article, ''))
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
CREATE INDEX IF NOT EXISTS idx_a012_document_no ON a012_wb_sales(document_no);

-- Verification queries (run manually after migration):
-- Check that all records have sale_id:
-- SELECT COUNT(*) FROM a012_wb_sales WHERE sale_id IS NULL OR sale_id = '';
-- Expected: 0

-- Check for duplicate sale_id:
-- SELECT sale_id, COUNT(*) as cnt FROM a012_wb_sales GROUP BY sale_id HAVING cnt > 1;
-- Expected: 0 rows (no duplicates)

-- Total record count:
-- SELECT COUNT(*) as total FROM a012_wb_sales;
