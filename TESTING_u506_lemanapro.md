# Testing Guide: LemanaPro Import (u506)

## Prerequisites

1. **LemanaPro Bearer Token**
   - Obtain Bearer token from LemanaPro B2B API
   - Token should have access to products endpoint

2. **Database**
   - Ensure database is initialized (`target/db/app.db`)
   - LemanaPro marketplace should exist in a005_marketplace
   - Create a connection in a006_connection_mp with LemanaPro credentials

## Setup Steps

### 1. Start Backend
```bash
cd crates/backend
cargo run
```

Backend should start on `http://localhost:3000`

### 2. Create LemanaPro Connection

Use the connection_mp API to create a connection:

```bash
# POST /api/connection_mp
curl -X POST http://localhost:3000/api/connection_mp \
  -H "Content-Type: application/json" \
  -d '{
    "code": "lemanapro-prod",
    "description": "LemanaPro Production",
    "marketplace_id": "mp-lemana",
    "organization": "Организация 1",
    "api_key": "YOUR_BEARER_TOKEN_HERE",
    "application_id": null,
    "secret_key": null,
    "username": null,
    "password": null,
    "active": true
  }'
```

Note the returned `id` - you'll need it for the import request.

### 3. Test Connection

```bash
# POST /api/connection_mp/test
curl -X POST http://localhost:3000/api/connection_mp/test \
  -H "Content-Type: application/json" \
  -d '{
    "code": "lemanapro-prod",
    "description": "LemanaPro Production",
    "marketplace_id": "mp-lemana",
    "organization": "Организация 1",
    "api_key": "YOUR_BEARER_TOKEN_HERE",
    "application_id": null,
    "active": true
  }'
```

Expected response:
```json
{
  "success": true,
  "message": "Подключение успешно",
  "details": "API LemanaPro доступен, токен валиден"
}
```

## Running Import

### 1. Start Import

```bash
curl -X POST http://localhost:3000/api/u506/import/start \
  -H "Content-Type: application/json" \
  -d '{
    "connection_id": "YOUR_CONNECTION_ID",
    "target_aggregates": ["a007_marketplace_product"],
    "mode": "interactive",
    "dateFrom": "2025-01-01",
    "dateTo": "2025-12-31"
  }'
```

Expected response:
```json
{
  "session_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "started",
  "message": "Импорт запущен"
}
```

### 2. Monitor Progress

```bash
# Replace SESSION_ID with the session_id from previous response
curl http://localhost:3000/api/u506/import/SESSION_ID/progress
```

Response example:
```json
{
  "session_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "running",
  "started_at": "2025-10-24T10:30:00Z",
  "updated_at": "2025-10-24T10:30:15Z",
  "completed_at": null,
  "aggregates": [
    {
      "aggregate_index": "a007_marketplace_product",
      "aggregate_name": "Товары маркетплейса",
      "status": "running",
      "processed": 150,
      "total": 5000,
      "inserted": 100,
      "updated": 50,
      "errors": 0,
      "current_item": "85087716 - Тепловая пушка электрическая"
    }
  ],
  "total_processed": 150,
  "total_inserted": 100,
  "total_updated": 50,
  "total_errors": 0,
  "errors": []
}
```

Continue polling this endpoint to monitor progress.

### 3. Check Completion

When import completes, status will change to `completed` or `completed_with_errors`:

```json
{
  "session_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "status": "completed",
  "started_at": "2025-10-24T10:30:00Z",
  "updated_at": "2025-10-24T10:45:30Z",
  "completed_at": "2025-10-24T10:45:30Z",
  "aggregates": [
    {
      "aggregate_index": "a007_marketplace_product",
      "aggregate_name": "Товары маркетплейса",
      "status": "completed",
      "processed": 5000,
      "total": 5000,
      "inserted": 3500,
      "updated": 1500,
      "errors": 0,
      "current_item": null
    }
  ],
  "total_processed": 5000,
  "total_inserted": 3500,
  "total_updated": 1500,
  "total_errors": 0,
  "errors": []
}
```

## Verify Results

### 1. Check Database

```bash
# List imported products
curl http://localhost:3000/api/marketplace_product \
  | jq '.[] | select(.marketplace_id == "mp-lemana") | {sku: .marketplace_sku, name: .product_name, brand: .brand}'
```

### 2. Check Logs

```bash
# Application logs
tail -f target/logs/backend.log | grep -i lemanapro

# API request logs
tail -f lemanapro_api_requests.log
```

## Testing Scenarios

### Scenario 1: First-Time Import
- All products should be inserted (inserted count = processed count)
- No updates
- All products should have `marketplace_id = "mp-lemana"`

### Scenario 2: Re-Import (Update)
- Run import again with same connection
- Most products should be updated
- Few or no inserts (unless new products added)

### Scenario 3: Invalid Token
- Use invalid Bearer token in connection
- Connection test should fail with 401 error
- Import should fail immediately

### Scenario 4: Network Error
- Disconnect network during import
- Import should fail gracefully
- Errors should be logged in progress

### Scenario 5: Large Import
- Import with large dataset (10k+ products)
- Monitor memory usage
- Verify pagination works correctly

## Expected Behavior

### Success Criteria
✅ Connection test passes with valid token
✅ Import starts and returns session_id
✅ Progress updates in real-time
✅ Products created/updated in database
✅ Import completes with correct statistics
✅ No memory leaks during large imports
✅ Logs show detailed API requests/responses

### Known Limitations
⚠️ **Price field is None** - Prices require separate API call
⚠️ **Stock not available** - Products API doesn't provide stock info
⚠️ **Category structure** - Only single level category mapped

## Troubleshooting

### Issue: Connection Test Fails
**Symptoms**: 401 Unauthorized error

**Solutions**:
1. Verify Bearer token is correct
2. Check token hasn't expired
3. Ensure token has access to products endpoint
4. Try test environment URL first

### Issue: Import Starts but No Progress
**Symptoms**: Status stays at "running" with 0 processed

**Solutions**:
1. Check `lemanapro_api_requests.log` for API errors
2. Verify network connectivity
3. Check backend logs for exceptions
4. Ensure LemanaPro API is accessible

### Issue: Products Not Appearing in Database
**Symptoms**: Import completes but products not found

**Solutions**:
1. Verify marketplace_id matches ("mp-lemana")
2. Check connection_id in import request
3. Query database directly: `SELECT * FROM a007_marketplace_product WHERE marketplace_id = 'mp-lemana'`
4. Check for database errors in backend logs

### Issue: Parse Errors
**Symptoms**: Import fails with JSON parsing errors

**Solutions**:
1. Check `lemanapro_api_requests.log` for actual API response
2. Verify API response structure matches expected schema
3. Look for API version changes
4. Check if optional fields are missing

## API Rate Limits

LemanaPro API may have rate limits (mentioned as 429 Too Many Requests in spec):
- If you hit rate limits, import will fail
- Consider adding delay between requests (not currently implemented)
- Check API documentation for specific limits

## Performance Tips

1. **Batch Size**: Default is 100 items per page. Can be increased up to 1500 if needed.
2. **Concurrent Requests**: Currently sequential. Can be parallelized if needed.
3. **Database Indexes**: Ensure index on `marketplace_sku` for fast lookups.

## Next Steps After Successful Import

1. **Implement Price Import** - Use `/b2bintegration/sale-prices/v1/sales-prices`
2. **Add Stock Tracking** - If available from other endpoints
3. **Schedule Imports** - Set up periodic imports (daily/hourly)
4. **Monitor Changes** - Use `If-Modified-Since` header for incremental updates

## Support

If issues persist:
1. Review `IMPLEMENTATION_SUMMARY_u506_lemanapro.md`
2. Check LemanaPro B2B API documentation
3. Examine `memory-bank/b2b.yaml` for API specification
4. Compare with working OZON import (u502) for reference


