-- Migration: Add status_norm and substatus_raw columns to a010_ozon_fbs_posting
-- Date: 2025-11-25
-- Purpose: Store status and substatus fields from state JSON for easier querying

-- Add new columns
ALTER TABLE a010_ozon_fbs_posting 
ADD COLUMN status_norm TEXT NOT NULL DEFAULT '';

ALTER TABLE a010_ozon_fbs_posting 
ADD COLUMN substatus_raw TEXT;

-- Add index for faster filtering by status
CREATE INDEX IF NOT EXISTS idx_a010_status_norm ON a010_ozon_fbs_posting(status_norm);

-- Update existing records to extract status_norm from state_json
UPDATE a010_ozon_fbs_posting
SET status_norm = json_extract(state_json, '$.status_norm')
WHERE status_norm = '';

-- Update existing records to extract substatus_raw from state_json
UPDATE a010_ozon_fbs_posting
SET substatus_raw = json_extract(state_json, '$.substatus_raw')
WHERE substatus_raw IS NULL;

-- Note: New records will be populated automatically through the repository upsert_document method

