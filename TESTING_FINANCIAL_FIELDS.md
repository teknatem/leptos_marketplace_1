# Testing Financial Fields Calculation

## Implementation Summary

The financial fields calculation has been implemented for a012_wb_sales documents. The calculation happens automatically during document posting and determines whether to use plan or fact values based on P903 WB Finance Report data availability.

## Changes Made

1. **service.rs**: Added `calculate_financial_fields()` function that:
   - Queries P903 by `srid` (document_no)
   - Filters for "Продажа" operations
   - Gets `acquiring_fee_pro` from marketplace
   - Calculates plan fields if no P903 data
   - Calculates fact fields if P903 data exists

2. **posting.rs**: Integrated financial calculation into `post_document()` before saving

3. **sales.rs**: Added initialization of all financial fields to None during import

## How to Test

### 1. Restart Backend

The backend needs to be restarted to load the new code:

```powershell
# Stop the current backend (Ctrl+C in the terminal)
# Then restart:
cargo run --bin backend
```

### 2. Test Plan Fields (No P903 Data)

Find a WB sales document that doesn't have P903 data:

```bash
# API Request to get a document
GET http://localhost:3000/api/a012/wb-sales/{document_id}

# Check if P903 has data for this srid
GET http://localhost:3000/api/p903/finance-report/search-by-srid?srid={srid}
```

If P903 returns empty, post the document:

```bash
POST http://localhost:3000/api/a012/wb-sales/{document_id}/post
```

Expected result:
- `is_fact = false`
- `sell_out_plan` = finishedPrice
- `acquiring_fee_plan` = acquiring_fee_pro * finishedPrice / 100
- `commission_plan` = finishedPrice - amount_line
- `supplier_payout_plan` = amount_line - acquiring_fee_plan
- `profit_plan` calculated correctly

### 3. Test Fact Fields (With P903 Data)

Find a WB sales document that has P903 data:

```bash
# Check P903 data exists
GET http://localhost:3000/api/p903/finance-report/search-by-srid?srid={srid}
```

If P903 returns data with `supplier_oper_name = "Продажа"`, unpost then re-post the document:

```bash
# Unpost first to clear
POST http://localhost:3000/api/a012/wb-sales/{document_id}/unpost

# Then post
POST http://localhost:3000/api/a012/wb-sales/{document_id}/post
```

Expected result:
- `is_fact = true`
- `sell_out_fact` = retail_amount (from P903)
- `acquiring_fee_fact` = acquiring_fee (from P903)
- `other_fee_fact` = rebill_logistic_cost (from P903)
- `commission_fact` = ppvz_vw + ppvz_vw_nds (from P903)
- `supplier_payout_fact` = ppvz_for_pay (from P903)
- `profit_fact` calculated correctly

### 4. Verify Calculations

Check the document details after posting:

```bash
GET http://localhost:3000/api/a012/wb-sales/{document_id}
```

Look for the financial fields in the response:
- All plan/fact fields should be populated
- `is_fact` should indicate which set is active
- Values should match the formulas

### 5. Check Logs

Monitor the backend logs for calculation messages:

```
Calculated PLAN fields for document {id} (srid: {srid})
```

or

```
Calculated FACT fields for document {id} (srid: {srid})
```

## Known Document IDs for Testing

From the terminal logs, these documents were recently accessed:
- `0cb69c76-dd76-4586-93c3-97bd368faac3`
- `f5f57710-3cdd-4ae3-87dd-c4e5233734a1`
- `c8fc8dcc-3d1b-455e-8855-64f95a60f102`
- `8e16093c-c710-4e62-8610-1a0a5e34236c`

## Batch Testing

To test multiple documents at once:

```bash
POST http://localhost:3000/api/a012/wb-sales/batch-post
{
  "ids": [
    "document_id_1",
    "document_id_2",
    "document_id_3"
  ]
}
```

## SQL Verification

You can also verify the fields directly in the database:

```sql
-- Check a specific document's financial fields
SELECT 
    id,
    document_no,
    is_fact,
    sell_out_plan,
    sell_out_fact,
    acquiring_fee_plan,
    acquiring_fee_fact,
    commission_plan,
    commission_fact,
    profit_plan,
    profit_fact
FROM a012_wb_sales
WHERE id = 'document_id';
```

## Troubleshooting

If calculations don't appear:
1. Verify the backend restarted with new code
2. Check logs for any errors during posting
3. Verify P903 data exists and is accessible
4. Check marketplace `acquiring_fee_pro` is set correctly
5. Ensure `finished_price` and `amount_line` have values

## Edge Cases Handled

- Missing P903 data → Uses plan calculations
- Multiple P903 entries → Aggregates values from "Продажа" operations only
- Missing marketplace → acquiring_fee_pro defaults to 0.0
- Missing prices → Uses 0.0 in calculations
- cost_of_production → Defaults to 0.0 if not set
