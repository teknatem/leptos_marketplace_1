# Database Migration: Add connection_mp_id to a007_marketplace_product

## Overview
Added a new mandatory field `connection_mp_id` to track which connection/cabinet was used to import each marketplace product.

## Database Changes Required

### Migration SQL
Execute the following SQL to add the new column to the database:

```sql
-- Add the connection_mp_id column (nullable initially for existing data)
ALTER TABLE a007_marketplace_product 
ADD COLUMN connection_mp_id TEXT;

-- Update existing records with a default or specific connection_mp_id
-- Option 1: Set to empty string (if you want to keep existing records)
UPDATE a007_marketplace_product 
SET connection_mp_id = '' 
WHERE connection_mp_id IS NULL;

-- Option 2: Delete existing records (if test data only)
-- DELETE FROM a007_marketplace_product;

-- Make the column NOT NULL after data migration
ALTER TABLE a007_marketplace_product 
ALTER COLUMN connection_mp_id SET NOT NULL;
```

### Alternative: Recreate Table
If the database contains only test data, you can drop and recreate the table, which will be handled automatically by the application on next run.

```sql
DROP TABLE IF EXISTS a007_marketplace_product;
```

## Code Changes Summary

### 1. Contracts Layer ✓
- Added `connection_mp_id: String` field to `MarketplaceProduct` aggregate
- Updated `new_for_insert()` and `new_with_id()` methods
- Updated `update()` method to handle connection_mp_id
- Added field to `MarketplaceProductDto`

### 2. Backend Repository ✓
- Added `connection_mp_id: String` to `Model` struct
- Updated `From<Model> for MarketplaceProduct` conversion
- Updated `insert()` and `update()` functions

### 3. Backend Service ✓
- Updated `create()` to pass connection_mp_id to new_for_insert
- Updated test data with placeholder connection_mp_id

### 4. Import Executors ✓
- Updated `u504_import_from_wildberries/executor.rs` to pass `connection.base.id.as_string()` when creating products
- Verified u501 doesn't create marketplace products (no changes needed)

### 5. Frontend UI - List View ✓
- Added `connection_mp_id` and `connection_mp_name` fields to `MarketplaceProductRow`
- Added `ConnectionMP` import and signal
- Created `connection_mp_map()` for cabinet name resolution
- Added fetch function `fetch_connections_mp()`
- Added "Кабинет" column in table UI
- Updated Excel export to include cabinet name
- Added cabinet name to search and sort functionality

### 6. Frontend UI - Details View ✓
- Added read-only "Кабинет" field displaying connection_mp_id
- Updated view_model to include connection_mp_id when loading data

## Testing Checklist

- [ ] Run database migration SQL
- [ ] Verify application starts without errors
- [ ] Test importing products from Wildberries - verify connection_mp_id is populated
- [ ] Check UI list displays cabinet names correctly
- [ ] Verify cabinet column is sortable and searchable
- [ ] Test Excel export includes cabinet name
- [ ] Check details view shows cabinet ID (read-only)
- [ ] Verify existing functionality still works (create, edit, delete products)

## Notes

- The `connection_mp_id` field is mandatory and automatically populated during import
- Manual product creation will require providing a connection_mp_id
- The field is read-only in the UI details form as it represents the data source
- Cabinet names are resolved dynamically from the a006_connection_mp table


