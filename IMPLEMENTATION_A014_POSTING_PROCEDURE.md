# A014 OZON Transactions Posting Procedure Implementation Summary

## Overview
Successfully implemented the posting procedure for A014 OZON Transactions list with projection creation to P904 Sales Data. The interface matches A010 OZON FBS Posting with checkboxes for selection and Post/Unpost buttons for batch operations.

## Changes Made

### Backend Changes

#### 1. Posting Module (`crates/backend/src/domain/a014_ozon_transactions/posting.rs`)
Created new posting module with two main functions:
- `post_document(id: Uuid)` - Sets `is_posted = true` and creates P904 projections
- `unpost_document(id: Uuid)` - Sets `is_posted = false` and deletes P904 projections

Logic follows the same pattern as A010/A011 posting modules.

#### 2. P904 Projection Builder (`crates/backend/src/projections/p904_sales_data/projection_builder.rs`)
Added `from_ozon_transactions()` function that:
- Finds related posting documents (A010 FBS or A011 FBO) by `posting_number`
- Distributes `header.accruals_for_sale` proportionally across posting lines based on their `amount_line`
- Uses `find_or_create_for_sale()` to auto-create/find `a007_marketplace_product` entries by SKU
- Retrieves `nomenclature_ref` from a007 mapping (doesn't create new nomenclature)
- Creates P904 entries with:
  - `date`: from `header.operation_date`
  - `registrator_ref`: document_id
  - `registrator_type`: "OZON_Transactions"
  - `connection_mp_ref`: from header
  - `nomenclature_ref` and `marketplace_product_ref`: from a007/a004 lookup
  - `customer_in`: proportional share of `accruals_for_sale`
  - `price_full` and `price_list`: from posting document lines
  - Other financial fields: 0.0 for now (to be filled later)

Falls back to creating basic entries if posting document not found.

#### 3. P904 Service (`crates/backend/src/projections/p904_sales_data/service.rs`)
Added `project_ozon_transactions()` function that calls the projection builder and persists entries.

#### 4. Handlers (`crates/backend/src/handlers/a014_ozon_transactions.rs`)
Added two new handlers:
- `post_document(Path(id))` - Calls `posting::post_document()`
- `unpost_document(Path(id))` - Calls `posting::unpost_document()`

#### 5. Routes (`crates/backend/src/main.rs`)
Registered two new routes:
- `POST /api/a014/ozon-transactions/:id/post`
- `POST /api/a014/ozon-transactions/:id/unpost`

#### 6. Module Registration (`crates/backend/src/domain/a014_ozon_transactions/mod.rs`)
Added `pub mod posting;` to expose the posting module.

### Frontend Changes

#### 7. List UI (`crates/frontend/src/domain/a014_ozon_transactions/ui/list/mod.rs`)
Enhanced the transactions list with:

**New State Management:**
- `selected_ids` - Tracks selected documents for batch operations
- `posting_in_progress` - Indicates when posting operations are running
- `operation_results` - Stores results of batch operations
- `current_operation` - Shows progress (current/total)

**Selection Features:**
- Checkbox column in table header with "select all/deselect all" toggle
- Individual checkbox in each row
- `toggle_selection()` - Toggles individual document selection
- `toggle_all()` - Selects/deselects all documents
- `is_selected()` - Checks if document is selected
- `all_selected()` - Checks if all documents are selected

**Batch Operation Buttons:**
- **Post** button - Posts selected documents sequentially
- **Unpost** button - Unposts selected documents sequentially
- Both buttons show count of selected documents: `Post (3)`, `Unpost (5)`
- Buttons are disabled when no documents selected or operation in progress

**Operation Handlers:**
- `post_selected()` - Iterates through selected IDs, calls POST endpoint for each
- `unpost_selected()` - Iterates through selected IDs, calls POST endpoint for each
- Both track results (success/failure) and reload list after completion

**UI Improvements:**
- All table cells (except checkbox column) have click handlers to open detail view
- Maintains existing sorting and filtering functionality

## Technical Details

### Proportional Distribution Algorithm
The system distributes `accruals_for_sale` from A014 across multiple items proportionally:

```rust
let total_amount: f64 = posting_lines.iter()
    .map(|l| l.amount_line.unwrap_or(0.0))
    .sum();

for line in &posting_lines {
    let proportion = if total_amount > 0.0 {
        line.amount_line.unwrap_or(0.0) / total_amount
    } else {
        1.0 / posting_lines.len() as f64
    };
    let customer_in = accruals_for_sale * proportion;
}
```

### Reference Chain
A014 → A010/A011 → A007 → A004
- A014 references A010/A011 by `posting_number`
- A010/A011 contains SKU and product details
- A007 (marketplace_product) is found/created by SKU
- A004 (nomenclature) is retrieved from A007 mapping

### Error Handling
- If posting document (A010/A011) not found, creates basic P904 entries with split `accruals_for_sale`
- Handles `Option<f64>` fields safely with `.unwrap_or(0.0)`
- Logs warnings when posting documents are missing

## Testing Checklist
- [x] Backend compiles without errors
- [x] Frontend compiles without errors
- [ ] Test posting single A014 document
- [ ] Verify P904 entries created with correct proportional amounts
- [ ] Test unposting removes P904 entries
- [ ] Test batch selection (select all, select individual)
- [ ] Test batch Post operation (multiple documents)
- [ ] Test batch Unpost operation (multiple documents)
- [ ] Verify SKU → A007 → A004 reference chain works
- [ ] Test with documents that have multiple items
- [ ] Test with documents where posting not found
- [ ] Verify UI updates after operations

## Database Migration
No database migration required. Uses existing tables:
- `a014_ozon_transactions` (already has `is_posted` column)
- `p904_sales_data` (already exists)
- `a007_marketplace_product` (already exists)
- `a004_nomenclature` (already exists)

## Files Modified
- `crates/backend/src/domain/a014_ozon_transactions/posting.rs` (new)
- `crates/backend/src/domain/a014_ozon_transactions/mod.rs`
- `crates/backend/src/projections/p904_sales_data/projection_builder.rs`
- `crates/backend/src/projections/p904_sales_data/service.rs`
- `crates/backend/src/handlers/a014_ozon_transactions.rs`
- `crates/backend/src/main.rs`
- `crates/frontend/src/domain/a014_ozon_transactions/ui/list/mod.rs`

## Next Steps
1. Test the posting functionality in the running application
2. Verify P904 projections are created correctly
3. Fill in additional financial fields in P904 as business logic is defined
4. Consider adding progress indicator modal for batch operations
5. Add error handling UI for failed operations

