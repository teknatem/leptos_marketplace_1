# 🔬 Wildberries API Diagnostic Guide

**Quick Start**: How to run and interpret the diagnostic tests

## 🚀 Running Diagnostics

### Step 1: Start Backend
```powershell
cd C:\dev\rust\marketplace\leptos_marketplace_1
cargo run --bin backend
```

### Step 2: Trigger Import
1. Open frontend in browser
2. Navigate to Wildberries import section
3. Select a connection
4. Click "Import"

### Step 3: Watch Console
The backend will automatically run diagnostics **before** the normal import.

## 📊 Reading the Results

### Console Output Format

```
╔═══════════════════════════════════════════════════════════
║ WILDBERRIES IMPORT DIAGNOSTICS
║ Connection: Your Connection Name (uuid)
╚═══════════════════════════════════════════════════════════
┌─────────────────────────────────────────────────────────
│ 🔬 RUNNING API DIAGNOSTICS
│ Testing different API request variations...
└─────────────────────────────────────────────────────────
┌─────────────────────────────────────────────────────────
│ 📊 DIAGNOSTIC RESULTS:
│
│ Test #1: Current implementation
│   ✓ SUCCESS
│   Items returned: 6
│   Cursor total: 6
│
│ Test #2: Increased limit to 1000
│   ✓ SUCCESS  
│   Items returned: 6
│   Cursor total: 6
│
│ Test #3: Minimal request (no settings)
│   ✓ SUCCESS
│   Items returned: 6
│   Cursor total: 6
│
│ Test #4: Empty textSearch filter
│   ✓ SUCCESS
│   Items returned: 6
│   Cursor total: 6
└─────────────────────────────────────────────────────────
```

## 🎯 Interpreting Results

### Scenario A: All Tests Return Same Low Count
```
│ 📌 CONCLUSION:
│ All tests return similar low counts (6)
│ This suggests:
│   1. These might be ALL products in this account, OR
│   2. Products have different status (archived, etc.), OR
│   3. API key has limited scope/permissions
```

**Action**: Check Wildberries personal account to verify:
1. Total number of active products
2. Status of each product (Active vs. Archived)
3. API key permissions

### Scenario B: One Test Returns Higher Count
```
│ 🔍 IMPORTANT FINDING:
│ Test 'Increased limit to 1000' returned cursor.total=950
│ This suggests there ARE more products available!
│ Current implementation might be using wrong parameters.
```

**Action**: We found the solution! The working test shows correct parameters.

### Scenario C: Tests Fail
```
│ Test #2: Increased limit to 1000
│   ✗ FAILED
│   Error: API returned status 400: Bad Request
```

**Action**: Check error message for clues about API requirements.

## 📝 Detailed Logs

All requests and responses are logged to:
```
wildberries_api_requests.log
```

### Log Format
```
========== DIAGNOSTIC TEST: Current implementation ==========
Request body: {"settings":{"cursor":{"total":0},"filter":{}},"limit":100}
Response status: 200 OK
Response headers: {...}
Response body: {"cards":[...],"cursor":{"total":6}}
✓ Success: 6 items, cursor.total=6
```

## ✅ Checklist: What to Verify

### In Wildberries Personal Account
- [ ] Total number of products shown in UI
- [ ] Filter settings (are you viewing "All" or just "Active"?)
- [ ] Status of each product:
  - [ ] Active
  - [ ] Archived
  - [ ] On Moderation
  - [ ] Blocked
- [ ] API key permissions:
  - [ ] Content (read)
  - [ ] Marketplace
  - [ ] Statistics
- [ ] API key creation date (is it recent?)
- [ ] Associated supplier/warehouse

### In Diagnostic Output
- [ ] Do all tests succeed?
- [ ] Are cursor.total values consistent?
- [ ] Does any test return higher count?
- [ ] Are there any error messages?

## 🔍 Common Findings

### Finding 1: Actually Only 6-10 Products
**Symptoms**: All tests return same low count  
**Cause**: Account really has few products  
**Solution**: Verify in Wildberries UI

### Finding 2: Archived Products Not Counted
**Symptoms**: UI shows 1000, API returns 10  
**Cause**: API only returns active products  
**Solution**: Activate archived products or accept limitation

### Finding 3: API Key Limited Scope
**Symptoms**: All tests return 401 or limited results  
**Cause**: API key doesn't have proper permissions  
**Solution**: Create new API key with full permissions

### Finding 4: Correct Parameters Found
**Symptoms**: One test returns much higher count  
**Cause**: We found the correct API parameters!  
**Solution**: Update implementation to use working parameters

## 🆘 Need Help?

### If ALL Tests Return Low Count (~6-10)
1. **First**: Check Wildberries personal account product count
2. **Second**: Verify product statuses (Active vs. Archived)
3. **Third**: Check API key permissions
4. **Last**: Contact Wildberries support with logs

### If Tests Return Different Counts
1. **Great!** We found a working configuration
2. Note which test succeeded
3. Share results for implementation

### If Tests Fail with Errors
1. Check error messages in console
2. Check `wildberries_api_requests.log` for details
3. Verify API key is valid
4. Verify internet connection
5. Share error logs for analysis

## 📧 What to Share

If you need help analyzing results, provide:

1. **Console output** (the diagnostic section)
2. **wildberries_api_requests.log** file
3. **Product count from Wildberries UI** (screenshot helpful)
4. **API key permissions** (don't share the actual key!)

## 🛠️ Technical Details

### Tests Performed

| Test # | Description | Request Structure |
|--------|-------------|-------------------|
| 1 | Current implementation | `{"settings":{"cursor":{"total":0},"filter":{}},"limit":100}` |
| 2 | Increased limit | `{"settings":{"cursor":{"total":0},"filter":{}},"limit":1000}` |
| 3 | Minimal request | `{"limit":1000}` |
| 4 | Empty textSearch | `{"settings":{"cursor":{"total":0},"filter":{}},"limit":1000}` |

### Response Analysis

For each test, we check:
- ✅ HTTP status code (should be 200)
- ✅ Response parseable as JSON
- ✅ `cursor.total` value
- ✅ Number of items in `cards` array
- ⚠️ Mismatch between cursor.total and cards.length

## 📚 Additional Resources

- **Full Investigation Report**: `docs/wildberries_api_investigation.md`
- **Testing Documentation**: `TESTING_u504_wildberries.md`
- **Diagnostic Summary**: `DIAGNOSTIC_SUMMARY_u504.md`

---

**Status**: ✅ Diagnostics implemented and ready to run  
**Next**: Run import and analyze results

