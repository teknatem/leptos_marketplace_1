# p903_wb_finance_report Field Inventory

This file is the working inventory for `p903_wb_finance_report`.

## Canonical purpose

`p903_wb_finance_report` is the raw fact projection for Wildberries finance report rows.

It serves three roles:

1. Source of fact GL rows for `sys_general_ledger`
2. Registrator-level source for fact drilldown
3. Carrier of nomenclature link for deep drilldown through `a004_nomenclature`

## Field groups

### Identity and routing

- `id`: stable primary key of the projection row; used as `registrator_ref` for GL
- `rr_dt`: business date of the finance row
- `rrd_id`: natural key from WB report row
- `source_row_ref`: external source identity
- `connection_mp_ref`: cabinet / connection scope
- `organization_ref`: legal entity scope
- `srid`: order / line identity for linked WB rows
- `a004_nomenclature_ref`: direct link to `a004_nomenclature.id` for drilldown
- `nm_id`: WB SKU/article key used to resolve `a004_nomenclature_ref`

### GL-posting critical fields

- `retail_amount`
- `return_amount`
- `acquiring_fee`
- `ppvz_vw`
- `ppvz_vw_nds`
- `ppvz_sales_commission`
- `rebill_logistic_cost`
- `storage_fee`
- `penalty`
- `ppvz_for_pay`
- `delivery_amount`
- `supplier_oper_name`
- `extra`

These fields are used directly by `general_ledger_builder.rs` to decide turnover, sign, and resource origin.

### Reference and descriptive fields

- `subject_name`
- `sa_name`
- `bonus_type_name`

These fields are currently useful for raw analysis and UI inspection, but they are not primary routing keys for GL.

### Raw finance attributes currently not driving runtime logic

- `acquiring_percent`
- `additional_payment`
- `commission_percent`
- `delivery_rub`
- `quantity`
- `retail_price`
- `retail_price_withdisc_rub`
- `cashback_amount`
- `ppvz_kvw_prc`
- `ppvz_kvw_prc_base`
- `srv_dbs`

These fields are present in the imported finance payload and populated in data, but current backend runtime does not use them as first-class routing keys.

## Suspicious points

- `a004_nomenclature_ref` must be filled primarily from `a007_marketplace_product`, but current WB data is historically inconsistent:
  product import stores `nm_id` in `a007.marketplace_sku`, while some sales/order flows created `a007` rows with `supplier_article` in the same field.
- Because of that inconsistency, `p903.nm_id -> a007.marketplace_sku` is the canonical rule, but legacy rows may require an exact fallback through `p903.sa_name -> a007.article`.
- `delivery_rub` is semantically important for marketplace logistics, but current `p903` runtime logic does not yet use it as a first-class deep-drilldown route.
- `additional_payment` and `cashback_amount` look financially meaningful, but there is no stable current contract in `p903` runtime about how they should appear in fact drilldown.
- `acquiring_percent`, `commission_percent`, `ppvz_kvw_prc`, `ppvz_kvw_prc_base` are analytic attributes, not posting keys. They should not drive GL drilldown routing.
- No field is marked safe-to-delete yet. Even low-value fields are still part of the raw WB payload contract and may be used by analysts or future mappings.

## Recommended direction

- Keep `p903` as the raw fact carrier.
- Use `sys_general_ledger` as the amount source for indicators and totals.
- Use `p903.a004_nomenclature_ref` only as the link to `a004_nomenclature` for deep drilldown.
- For new WB finance imports, resolve `a004_nomenclature_ref` at write time from `a007_marketplace_product` first; use `p908` only as a legacy backfill fallback.
- Do not remove columns from `p903` until there is a confirmed business-level schema for the WB finance import payload.
