# General Ledger And Analytics Projections

`general_ledger` is the accounting layer. It stores the posting header: business date, layer, turnover code, debit account, credit account, amount, registrator and a link to the analytical detail.

Analytical projections keep the decomposed business rows:
- `p909_mp_order_line_turnovers` stores turnovers by marketplace order line.
- `p910_mp_unlinked_turnovers` stores turnovers from finance rows that cannot be linked to an order line.
- `p911_wb_advert_by_items` stores WB advertising expenses by item.

Linking rules:
- `p909` and `p910` use `general_ledger_ref` as a direct reference to one `general_ledger` row. For these projections `general_ledger.detail_kind` points to the projection key and `general_ledger.detail_id` stores the analytical row id.
- `p911` uses grouped detail. Many `p911` rows may share one `general_ledger_ref`. For this case `general_ledger.detail_kind = p911_wb_advert_by_items` and `general_ledger.detail_id = general_ledger.id`, because the detail page is opened by the shared ledger reference and then loads all rows in the group.

Expected invariants:
- every non-null `p909/p910/p911.general_ledger_ref` must reference an existing `sys_general_ledger.id`;
- every `sys_general_ledger.detail_kind` must resolve to an existing analytical detail target;
- for `p911`, the amount of one `general_ledger` row must equal the sum of all `p911.amount` rows with the same `general_ledger_ref`.
