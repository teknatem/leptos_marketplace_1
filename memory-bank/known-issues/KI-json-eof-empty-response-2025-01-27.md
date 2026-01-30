---
type: known-issue
date: 2025-01-27
tags: [json, error, debugging, frontend, backend]
severity: medium
---

# Known Issue: JSON "EOF while parsing" Usually Means Empty Backend Response

## Symptom

Frontend displays error:
```
EOF while parsing a value at line 1 column 0
```

## Root Cause

This JSON parsing error typically indicates:
1. Backend returned empty body (no JSON at all)
2. Backend returned HTTP error status (4xx/5xx) without JSON body
3. Backend endpoint doesn't exist or routing failed

## Detection

1. Check browser Network tab for actual response
2. Check backend logs for errors
3. Look for `StatusCode::INTERNAL_SERVER_ERROR` or `StatusCode::NOT_FOUND` returns without JSON body

## Fix

1. Check backend handler is returning proper JSON response
2. Verify all required validations pass (schema exists, data source supported, etc.)
3. Ensure service functions accept correct parameters

## Example from This Session

Service was checking:
```rust
if config.data_source != P903_SCHEMA.id {
    return Err(anyhow::anyhow!("Unsupported data source"));
}
```

But frontend was sending `s001_wb_finance` instead of `p903_wb_finance_report`.

## Prevention

- When adding new schema IDs, update ALL validation checks
- Add logging before returning error responses
- Consider returning JSON error bodies instead of bare StatusCode
