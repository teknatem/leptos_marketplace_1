-- Migration: Add posting_ref and posting_ref_type to a014_ozon_transactions
-- Date: 2025-11-25
-- Purpose: Store reference to posting document (A010 FBS or A011 FBO) in transactions

-- Add new columns for posting reference
ALTER TABLE a014_ozon_transactions 
ADD COLUMN posting_ref TEXT,
ADD COLUMN posting_ref_type TEXT;

-- Add index for faster lookup by posting_ref
CREATE INDEX IF NOT EXISTS idx_a014_posting_ref ON a014_ozon_transactions(posting_ref);

-- Add comment describing the columns
-- posting_ref: UUID reference to A010 OZON FBS Posting or A011 OZON FBO Posting document
-- posting_ref_type: Type of posting document - "A010" for FBS, "A011" for FBO

