---
type: session-debrief
date: 2025-01-27
tags: [pivot, schema, backend, json-error]
related: [s001_wb_finance, p903_wb_finance_report, SchemaRegistry]
---

# Session Debrief: Pivot Schema ID Mismatch

## Summary

User reported "EOF while parsing a value at line 1 column 0" error on the pivot_dashboard page. This JSON parsing error indicated the backend was returning empty responses because the service layer was hardcoded to only accept the old schema ID `p903_wb_finance_report`, while the new universal pivot system uses `s001_wb_finance`.

## Main Difficulties

1. **Symptom vs Root Cause**: The frontend error "EOF while parsing" looked like a JSON issue but was actually a backend returning 500/404 errors
2. **Hardcoded Schema References**: Multiple service functions (`execute_dashboard`, `generate_sql`, `get_distinct_values`) were hardcoded to check for `P903_SCHEMA.id`
3. **Function Signature Mismatch**: `get_distinct_values` didn't accept `schema_id` parameter, making it impossible to work with dynamic schemas

## Resolutions

1. Updated `execute_dashboard` in `service.rs`:
   - Added check for both `p903_wb_finance_report` AND `s001_wb_finance`
   - Uses registry to validate schema exists

2. Updated `generate_sql` similarly

3. Refactored `get_distinct_values`:
   - Added `schema_id` parameter
   - Uses `get_registry().get_schema()` to find schema
   - Uses `get_registry().get_table_name()` for dynamic table lookup

4. Updated handler `d401_wb_finance.rs`:
   - Now passes `schema_id` to `service::get_distinct_values()`

## Files Modified

- `crates/backend/src/dashboards/d401_wb_finance/service.rs`
- `crates/backend/src/api/handlers/d401_wb_finance.rs`

## TODO / Open Questions

- [ ] Fully refactor service to use dynamic schema lookup instead of hardcoded P903_SCHEMA references
- [ ] Consider renaming d401_wb_finance handler/service to pivot_service for clarity
- [ ] The `schema_owned` variable is unused in `execute_dashboard` - opportunity for full dynamic schema support

## Links

- [[KI-json-eof-empty-response-2025-01-27]]
- [[LL-schema-registry-migration-2025-01-27]]
