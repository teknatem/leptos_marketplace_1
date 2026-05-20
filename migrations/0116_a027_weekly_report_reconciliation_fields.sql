-- a027 weekly report reconciliation fields are stored in weekly_report_manual_json.
-- Keep old rows readable and normalize malformed/empty JSON before the Rust DTO
-- starts serializing the new optional fields.

UPDATE a027_wb_documents
SET weekly_report_manual_json = '{}'
WHERE weekly_report_manual_json IS NULL
   OR trim(weekly_report_manual_json) = ''
   OR json_valid(weekly_report_manual_json) = 0;

UPDATE a027_wb_documents
SET weekly_report_manual_json = json_set(
    weekly_report_manual_json,
    '$.other_deductions', json_extract(weekly_report_manual_json, '$.other_deductions'),
    '$.logistics', json_extract(weekly_report_manual_json, '$.logistics'),
    '$.acquiring', json_extract(weekly_report_manual_json, '$.acquiring')
)
WHERE json_valid(weekly_report_manual_json) = 1;
