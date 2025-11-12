-- Migration script to update existing a010_ozon_fbs_posting records
-- This updates the state_json to include substatus_raw field

-- Note: Since state is stored as JSON, new substatus_raw field will be
-- automatically included when documents are re-imported or updated.
-- No schema changes are needed.

-- For existing records, substatus_raw will be null until they are re-imported.
-- To force re-import, you can:
-- 1. Delete all existing FBS postings:
--    DELETE FROM a010_ozon_fbs_posting;
-- 2. Re-run the import from OZON API

-- Or update existing JSON to add substatus_raw:
-- This query would need to be executed per record with proper JSON manipulation
-- SQLite has limited JSON support, so it's easier to just re-import the data

SELECT 'Migration note: state_json will automatically include substatus_raw for new/updated records' AS message;
SELECT 'To add substatus_raw to existing records, re-import data from OZON API' AS recommendation;
