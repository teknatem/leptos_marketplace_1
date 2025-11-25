# Implementation Summary: A014 Substatus Column

## Date: 2025-11-25

## Overview
Added a "Substatus" column to the OZON Transactions (A014) list view, displaying substatus information from both FBS (A010) and FBO (A011) posting documents. Updated the "Дата доставки" (Delivery Date) column to fetch data from both posting types.

## Changes Made

### 1. Backend - A010 FBS Posting Repository
**File**: `crates/backend/src/domain/a010_ozon_fbs_posting/repository.rs`

- Added `status_norm: String` and `substatus_raw: Option<String>` fields to the `Model` struct
- Updated `upsert_document` method to save both fields when inserting or updating documents:
  - `status_norm: Set(aggregate.state.status_norm.clone())`
  - `substatus_raw: Set(aggregate.state.substatus_raw.clone())`

### 2. Backend - A011 FBO Posting Repository
**File**: `crates/backend/src/domain/a011_ozon_fbo_posting/repository.rs`

- Added `get_by_document_nos()` batch lookup method for retrieving multiple FBO postings at once
- Implementation follows the same pattern as the existing A010 method
- Filters out deleted records and returns a Vec of aggregates

### 3. Backend - A014 Transactions Service
**File**: `crates/backend/src/domain/a014_ozon_transactions/service.rs`

Updated three key functions to fetch data from both FBS and FBO postings:

#### `to_list_dto()`
- Added `substatus: Option<String>` parameter
- Updated DTO construction to include substatus field

#### `list_all_as_dto()`
- Fetches postings from both A010 (FBS) and A011 (FBO)
- Creates a combined lookup map: `posting_number -> (substatus, delivering_date)`
- Handles data from both posting types

#### `list_with_filters_as_dto()`
- Same approach as `list_all_as_dto()`
- Fetches from both posting types and creates combined lookup map

#### `get_by_posting_number_as_dto()`
- Tries FBS first, then FBO
- Returns substatus and delivering_date from whichever posting type is found

### 4. Contracts - A014 DTO
**File**: `crates/contracts/src/domain/a014_ozon_transactions/aggregate.rs`

- Added `substatus: Option<String>` field to `OzonTransactionsListDto`
- Positioned before `delivering_date` to match UI column order

### 5. Frontend - A014 DTO
**File**: `crates/frontend/src/domain/a014_ozon_transactions/ui/list/mod.rs`

- Added `substatus: Option<String>` field to frontend `OzonTransactionsDto`

### 6. Frontend - A014 List UI
**File**: `crates/frontend/src/domain/a014_ozon_transactions/ui/list/mod.rs`

#### Sorting
- Added "substatus" case to `Sortable::compare_by_field()` implementation
- Handles None values properly (None < Some)
- Case-insensitive string comparison

#### Table Header
- Added new column header: `<th>"Substatus"</th>`
- Positioned before "Дата Доставки" column
- Includes sort indicator

#### Table Body
- Added substatus cell displaying `item.substatus` or empty string if None
- Positioned before delivery date column

#### CSV Export
- Updated CSV header to include "Substatus" column
- Added substatus value to each row in export
- Properly escaped quotes in substatus strings

## Data Flow

1. **A010/A011 Import**: When FBS/FBO postings are imported from OZON API, `substatus_raw` is saved to the database
2. **A014 List Query**: When listing transactions:
   - Collects unique posting_numbers from transactions
   - Fetches matching postings from both A010 and A011 repositories (batch query)
   - Creates lookup map with substatus and delivery date
   - Joins data when building DTOs
3. **Frontend Display**: Substatus column shows the value from the posting document

## Key Features

- **Dual Source Support**: Handles both FBS (A010) and FBO (A011) posting types
- **Batch Queries**: Efficient lookup using `get_by_document_nos()` to minimize database calls
- **Null Handling**: Gracefully displays empty string when substatus is not available
- **Sortable**: Substatus column is fully sortable like other columns
- **Excel Export**: Substatus included in CSV export with proper formatting

## Database Schema

### A010 FBS Posting Migration
Added columns to `a010_ozon_fbs_posting` table:
- `status_norm` (TEXT, NOT NULL, default '') - extracted from state_json
- `substatus_raw` (TEXT, nullable) - extracted from state_json

Migration file: `migrate_a010_add_substatus.sql`
Applied to: `E:\dev\rust\leptos_marketplace_1\target\db\app.db`

### A011 FBO Posting
Already has `substatus_raw` and `status_norm` columns in table (no migration needed)

## Testing Notes

To verify the implementation:
1. Import OZON FBS/FBO postings to populate substatus_raw
2. View A014 list - Substatus column should appear before "Дата Доставки"
3. Test sorting by Substatus column
4. Export to Excel and verify Substatus column is included
5. Check both FBS and FBO transactions display substatus correctly

## Related Files

- Migration script: `migrate_substatus.sql` (informational only - no schema changes needed)
- Posting reference migration: `migrate_a014_posting_ref.sql`

