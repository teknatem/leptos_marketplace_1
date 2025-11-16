-- Migration: Refactor a007_marketplace_product aggregate
-- Description: Remove obsolete fields and standardize reference field naming
-- Date: 2025-11-16

-- Step 1: Drop obsolete columns
ALTER TABLE a007_marketplace_product DROP COLUMN marketplace_url;
ALTER TABLE a007_marketplace_product DROP COLUMN price;
ALTER TABLE a007_marketplace_product DROP COLUMN stock;
ALTER TABLE a007_marketplace_product DROP COLUMN product_name;

-- Step 2: Rename reference fields to use _ref suffix
ALTER TABLE a007_marketplace_product RENAME COLUMN marketplace_id TO marketplace_ref;
ALTER TABLE a007_marketplace_product RENAME COLUMN art TO article;
ALTER TABLE a007_marketplace_product RENAME COLUMN nomenclature_id TO nomenclature_ref;
ALTER TABLE a007_marketplace_product RENAME COLUMN connection_mp_id TO connection_mp_ref;

