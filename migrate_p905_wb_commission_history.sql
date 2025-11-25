-- Migration: Create p905_wb_commission_history table
-- Description: Store historical commission data from Wildberries by category

CREATE TABLE IF NOT EXISTS p905_wb_commission_history (
    id TEXT PRIMARY KEY NOT NULL,
    date TEXT NOT NULL,
    subject_id INTEGER NOT NULL,
    subject_name TEXT NOT NULL,
    parent_id INTEGER NOT NULL,
    parent_name TEXT NOT NULL,
    kgvp_booking REAL NOT NULL,
    kgvp_marketplace REAL NOT NULL,
    kgvp_pickup REAL NOT NULL,
    kgvp_supplier REAL NOT NULL,
    kgvp_supplier_express REAL NOT NULL,
    paid_storage_kgvp REAL NOT NULL,
    raw_json TEXT NOT NULL,
    loaded_at_utc TEXT NOT NULL,
    payload_version INTEGER NOT NULL DEFAULT 1
);

-- Create unique index to prevent duplicates per date and subject
CREATE UNIQUE INDEX IF NOT EXISTS idx_p905_date_subject 
    ON p905_wb_commission_history(date, subject_id);

-- Create index for date range queries
CREATE INDEX IF NOT EXISTS idx_p905_date 
    ON p905_wb_commission_history(date);

-- Create index for subject_id lookups
CREATE INDEX IF NOT EXISTS idx_p905_subject_id 
    ON p905_wb_commission_history(subject_id);

