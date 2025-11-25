# P905 WB Commission History - Implementation Summary

## Overview

Successfully implemented a complete projection (P905) for storing and managing Wildberries commission tariff history with automatic API synchronization, full CRUD operations, and comprehensive UI.

## ✅ Completed Features

### 1. Database Schema

**Table: `p905_wb_commission_history`**

Created in `crates/backend/src/shared/data/db.rs` with automatic initialization:

- `id` (TEXT PRIMARY KEY) - UUID
- `date` (TEXT) - Date in YYYY-MM-DD format
- `subject_id` (INTEGER) - Wildberries category ID
- `subject_name` (TEXT) - Category name
- `parent_id` (INTEGER) - Parent category ID
- `parent_name` (TEXT) - Parent category name
- Commission rates (6 fields):
  - `kgvp_booking` (REAL)
  - `kgvp_marketplace` (REAL)
  - `kgvp_pickup` (REAL)
  - `kgvp_supplier` (REAL)
  - `kgvp_supplier_express` (REAL)
  - `paid_storage_kgvp` (REAL)
- `raw_json` (TEXT) - Full JSON for change detection
- `loaded_at_utc` (TEXT) - Timestamp
- `payload_version` (INTEGER) - Version tracking

**Indexes:**

- UNIQUE(date, subject_id) - Prevents duplicates per date
- idx_p905_date - Fast date range queries
- idx_p905_subject_id - Fast category lookups

**Migration SQL:** `migrate_p905_wb_commission_history.sql`

### 2. Backend Implementation

#### Projection Repository

**File:** `crates/backend/src/projections/p905_wb_commission_history/repository.rs`

Functions:

- `upsert_entry()` - Insert/update commission record
- `list_with_filters()` - Query with date range, subject_id, sorting, pagination
- `get_latest_by_subject()` - Get most recent record for category
- `get_by_id()` - Fetch single record
- `delete_by_id()` - Remove record
- `get_all_subject_ids()` - Get unique categories

#### API Client

**File:** `crates/backend/src/usecases/u504_import_from_wildberries/wildberries_api_client.rs`

Added:

- `CommissionTariffRow` struct - Matches WB API response
- `CommissionTariffResponse` struct - API wrapper
- `fetch_commission_tariffs()` - GET public endpoint (no auth)
  - URL: `https://common-api.wildberries.ru/api/v1/tariffs/commission?locale=ru`

#### Sync Logic

**File:** `crates/backend/src/usecases/u504_import_from_wildberries/executor.rs`

Added `sync_commission_tariffs()` method:

1. Fetches all tariffs from WB API
2. Gets unique category_ids from `a007_marketplace_product`
3. Filters API results to only our categories
4. For each category:
   - Gets latest record
   - Compares raw JSON
   - Creates new record if different or missing
   - Skips if unchanged
5. Returns (new_count, updated_count, skipped_count)

#### HTTP Handlers

**File:** `crates/backend/src/handlers/p905_wb_commission_history.rs`

Endpoints:

- `GET /api/p905-commission/list` - List with filters
- `GET /api/p905-commission/:id` - Get single record
- `POST /api/p905-commission` - Create new record
- `PUT /api/p905-commission/:id` - Update existing (via save_commission)
- `DELETE /api/p905-commission/:id` - Delete record
- `POST /api/p905-commission/sync` - Trigger API synchronization

All registered in `crates/backend/src/main.rs`

### 3. Contracts/DTOs

**File:** `crates/contracts/src/projections/p905_wb_commission_history/dto.rs`

DTOs:

- `CommissionHistoryDto` - Full record representation
- `CommissionListRequest` - Query filters
- `CommissionListResponse` - Paginated results
- `CommissionSaveRequest` - Create/update payload
- `CommissionSaveResponse` - Save confirmation
- `CommissionSyncResponse` - Sync results with counts
- `CommissionDeleteResponse` - Delete confirmation

### 4. Frontend Implementation

#### API Client

**File:** `crates/frontend/src/projections/p905_wb_commission_history/api.rs`

Functions:

- `list_commissions()` - Fetch records with filters
- `get_commission()` - Get single record
- `save_commission()` - Create/update
- `delete_commission()` - Remove record
- `sync_commissions()` - Trigger API sync

#### List View

**File:** `crates/frontend/src/projections/p905_wb_commission_history/ui/list/mod.rs`

Features:

- **Filters:**
  - Date range (from/to)
  - Subject ID search
- **Table columns:**
  - Date, Subject ID, Category Name, Parent Name
  - 4 main commission rates (Booking, Marketplace, Pickup, Supplier)
- **Actions:**
  - 🔄 Refresh data
  - 🔄 Sync with API (calls backend sync endpoint)
  - - Create new record
  - Edit/Delete per row
- Real-time sync status display
- Pagination support

#### Details/Form View

**File:** `crates/frontend/src/projections/p905_wb_commission_history/ui/details/mod.rs`

Features:

- **Two tabs:**
  1. **Fields Tab** - Individual form inputs for all fields:
     - Date picker
     - Subject ID & name
     - Parent ID & name
     - 6 commission rate fields (with decimal precision)
  2. **Raw JSON Tab** - Direct JSON editing with validation
- Form validation (numbers, required fields)
- Save button on both tabs
- Success/error message display
- Auto-close tab after successful save

### 5. Navigation & Routing

#### Navigation Menu

**File:** `crates/frontend/src/layout/left/navbar.rs`

Added to "Регистры (Projections)" section:

- Label: "WB Commission History (P905)"
- Icon: percent
- Key: "p905_commission_history"

#### Routes

**File:** `crates/frontend/src/layout/center/tabs/tabs.rs`

Routes:

- `p905_commission_history` → List view
- `p905-commission/:id` → Edit form
- `p905-commission-new` → Create form

## Key Features

### 1. Smart Synchronization

- Only syncs categories that exist in our marketplace products
- Compares JSON to detect changes
- Avoids duplicate records on re-sync
- Tracks new/updated/skipped counts

### 2. Historical Tracking

- Preserves history of commission rate changes
- Date-based versioning
- Full raw JSON stored for audit trail

### 3. Full CRUD Operations

- Create records manually via UI
- Edit existing records
- Delete records
- List with comprehensive filtering

### 4. User-Friendly UI

- Clean, modern interface
- Date range filters
- Real-time sync status
- Tabbed form (fields vs JSON)
- Inline edit/delete actions
- Confirmation dialogs for destructive actions

## Files Created

### Backend

- `crates/backend/src/projections/p905_wb_commission_history/mod.rs`
- `crates/backend/src/projections/p905_wb_commission_history/repository.rs`
- `crates/backend/src/handlers/p905_wb_commission_history.rs`
- `migrate_p905_wb_commission_history.sql`

### Contracts

- `crates/contracts/src/projections/p905_wb_commission_history/mod.rs`
- `crates/contracts/src/projections/p905_wb_commission_history/dto.rs`

### Frontend

- `crates/frontend/src/projections/p905_wb_commission_history/mod.rs`
- `crates/frontend/src/projections/p905_wb_commission_history/api.rs`
- `crates/frontend/src/projections/p905_wb_commission_history/ui/mod.rs`
- `crates/frontend/src/projections/p905_wb_commission_history/ui/list/mod.rs`
- `crates/frontend/src/projections/p905_wb_commission_history/ui/details/mod.rs`

## Files Modified

### Backend

- `crates/backend/src/projections/mod.rs` - Added p905 module
- `crates/backend/src/handlers/mod.rs` - Added p905 handler
- `crates/backend/src/main.rs` - Registered routes
- `crates/backend/src/shared/data/db.rs` - Added table creation
- `crates/backend/src/usecases/u504_import_from_wildberries/wildberries_api_client.rs` - Added fetch method
- `crates/backend/src/usecases/u504_import_from_wildberries/executor.rs` - Added sync logic

### Contracts

- `crates/contracts/src/projections/mod.rs` - Added p905 module

### Frontend

- `crates/frontend/src/projections/mod.rs` - Added p905 module
- `crates/frontend/src/layout/left/navbar.rs` - Added nav item
- `crates/frontend/src/layout/center/tabs/tabs.rs` - Added routes

## Testing Instructions

### 1. Start Application

```bash
# Start backend
cargo run

# Start frontend (in another terminal)
cd crates/frontend
trunk serve
```

### 2. Test API Sync

1. Open browser to application
2. Navigate to "WB Commission History (P905)" in left menu
3. Click "🔄 Синхронизировать с API" button
4. Verify sync status message shows counts
5. Check table populates with commission data

### 3. Test Filtering

1. Set date range filters
2. Enter Subject ID
3. Click "🔄 Обновить"
4. Verify filtered results

### 4. Test CRUD

**Create:**

1. Click "+ Создать"
2. Fill all fields
3. Click "💾 Сохранить"
4. Verify success message
5. Return to list, verify new record appears

**Edit:**

1. Click "Изменить" on any row
2. Modify fields
3. Save
4. Verify changes in list

**Delete:**

1. Click "Удалить" on any row
2. Confirm dialog
3. Verify record removed

### 5. Test JSON Tab

1. Edit any record
2. Switch to "Raw JSON" tab
3. Modify JSON
4. Save
5. Verify changes applied

### 6. Verify Backend

```bash
# Check database
sqlite3 marketplace.db
SELECT COUNT(*) FROM p905_wb_commission_history;
SELECT * FROM p905_wb_commission_history LIMIT 5;

# Check logs
# Should see sync messages with counts
```

## API Endpoint

**Wildberries Commission Tariffs:**

- URL: `https://common-api.wildberries.ru/api/v1/tariffs/commission?locale=ru`
- Method: GET
- Auth: None (public endpoint)
- Returns: JSON with array of tariff objects

## Notes

- Commission rates are stored as percentages (e.g., 22.5 for 22.5%)
- Only categories that exist in `a007_marketplace_product.category_id` are synced
- JSON comparison prevents duplicate records on repeated syncs
- Date field uses YYYY-MM-DD format
- All timestamps in UTC
- Unique constraint on (date, subject_id) prevents duplicates

## Status

✅ **COMPLETED** - All todos completed successfully:

1. ✅ Database schema and migration SQL
2. ✅ Backend projection repository with CRUD
3. ✅ WB commission API client method
4. ✅ Sync logic with category filtering
5. ✅ HTTP handlers and route registration
6. ✅ DTOs and contracts
7. ✅ Frontend list UI with filters and sync
8. ✅ Frontend form with fields and JSON tabs
9. ✅ Navigation menu and routing
10. ✅ Code quality check (no linter errors)

The implementation is production-ready and follows all established patterns in the codebase.
