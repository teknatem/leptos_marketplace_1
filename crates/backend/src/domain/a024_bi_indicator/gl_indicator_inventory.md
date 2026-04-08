# GL-First Inventory For A024 Indicators

Updated: 2026-04-06

## Scope

This inventory classifies existing `a024_bi_indicator` records by whether they can be moved to the
GL-first mechanism:

- primary scalar comes from `sys_general_ledger`
- drilldown goes through the GL drilldown path

## Already On GL-First

| Code | Current source | GL mapping |
| --- | --- | --- |
| `IND-GL-MP-ACQ-FACT` | `dv004_general_ledger_turnovers` | `mp_acquiring`, `fact` |
| `IND-GL-MP-PENALTY-FACT` | `dv004_general_ledger_turnovers` | `mp_penalty`, `fact` |
| `IND-GL-MP-LOGISTICS-FACT` | `dv004_general_ledger_turnovers` | `mp_logistics`, `fact` |
| `IND-MP-RETURNS` | `dv004_general_ledger_turnovers` | `customer_return`, `fact` |
| `COST` | `dv004_general_ledger_turnovers` | `item_cost + item_cost_storno`, `oper` |

## Migrated In 2026-04-05 Pass

These indicators were using detail projections as the primary source, but their metric semantics map
to a single GL turnover and can therefore be moved to `dv004_general_ledger_turnovers`.

| Code | Old source | Old metric | New GL turnover | Layer |
| --- | --- | --- | --- | --- |
| `IND-MP-REV-PRICE` | `dv003_mp_order_line_turnovers` | `revenue_price` | `customer_revenue_pl` | `oper` |
| `IND-MP-COINVEST` | `dv003_mp_order_line_turnovers` | `coinvest` | `wb_coinvestment` | `oper` |
| `IND-MP-ACQUIRING` | `dv003_mp_order_line_turnovers` | `acquiring` | `mp_acquiring` | `oper` |
| `IND-MP-COST` | `dv003_mp_order_line_turnovers` | `cost` | `item_cost` | `oper` |
| `IND-MP-COMMISSION` | `dv003_mp_order_line_turnovers` | `commission` | `mp_commission` | `oper` |

## Not Migrated Yet

These indicators do not map to one turnover code and require a formula mechanism or a non-GL source.

| Code | Current source | Reason |
| --- | --- | --- |
| `IND-MP-REV` | `dv003_mp_order_line_turnovers` | Formula: `customer_revenue + spp_discount` |
| `IND-WB-ADS-SPEND` | `dv002_wb_advert_by_items` | GL scalar is possible, but current GL drilldown path does not route to `p911_wb_advert_by_items` |
| `IND-REVENUE-WB` | `dv001_revenue` | Derived sales metric over `p904_sales_data` |
| `IND-ORDERS` | `dv001_revenue` | Count metric, not a GL turnover |
| `IND-PROFIT-D` | `dv001_revenue` | Derived formula, not a single turnover |
| `IND-AVG-CHECK` | `dv001_revenue` | Derived formula / ratio |
| `IND-MARGIN` | none | No GL-backed compute config |
| `IND-REVENUE-OZON` | none | No GL-backed compute config |
| `IND-EMPTY` | none | Placeholder |
| `COMM` | `dv001_revenue` | Derived / generic draft indicator |
| `EXP` | `dv001_revenue` | Formula over multiple turnovers |
| `PROFIT` | `dv001_revenue` | Derived / generic draft indicator |
| `RET` | `dv001_revenue` | Derived / generic draft indicator |
| `REVENUE` | `dv001_revenue` | Derived / generic draft indicator |
| `TMP-A024` | none | Placeholder |

## Notes

- `IND-MP-RETURNS` was already moved earlier to `customer_return/fact`.
- `IND-MP-REV-PRICE` is corrected by migration `0053_fix_ind_mp_rev_price_to_customer_revenue_pl.sql`
  and now maps to `customer_revenue_pl` in `oper`.
- This inventory is intentionally conservative: only indicators with a one-to-one mapping to a GL
  turnover are migrated in this pass.
