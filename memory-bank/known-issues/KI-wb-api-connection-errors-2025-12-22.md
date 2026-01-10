---
title: Wildberries API Connection Errors
date: 2025-12-22
detection: "Import failed: error sending request for url (https://statistics-api.wildberries.ru/...)"
---

# Known Issue: Wildberries API Connection Errors

## Description

Users reported "error sending request" when attempting to import Sales or Finance reports from Wildberries.

## Causes

1. **Domain Transition**: Older code might have used `wb.ru` subdomains. WB migrated to `wildberries.ru` in early 2025.
2. **Strict Filtering**: WB servers may reject requests without a `User-Agent` header or with unconventional headers.
3. **System Proxies**: On Windows, system-level proxies can sometimes intercept and break WASM/Backend requests to Russian domains.

## Fix Applied (2025-12-22)

- Updated all WB endpoints to `https://statistics-api.wildberries.ru`.
- Added a standard Chrome `User-Agent`.
- Forced `.no_proxy()` in the client builder.
- Implemented detailed error reporting to distinguish between timeouts and DNS failures.

## Verification

Check `wildberries_api_requests.log` for samples. Success is marked by a 200 OK and a valid JSON body preview.



