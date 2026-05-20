use anyhow::Result;
use chrono::Utc;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::domain::a026_wb_advert_daily::aggregate::{
    WbAdvertDaily, WbAdvertFoundOrder, WbAdvertLinkedOrdersByNm,
};
use contracts::domain::common::AggregateId;
use contracts::shared::analytics::TurnoverLayer;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use super::repository;
use crate::domain::a015_wb_orders;
use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::shared::data::db::get_connection;

const REGISTRATOR_TYPE: &str = "a026_wb_advert_daily";
const TURNOVER_CODE_RESERVE: &str = "advert_clicks_order_accrual";
const TURNOVER_CODE_DIRECT: &str = "advert_clicks_no_order";
const RESOURCE_TABLE_P913: &str = "p913_wb_advert_order_attr";
const RESOURCE_TABLE_P911: &str = "p911_wb_advert_by_items";

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn to_general_ledger_entry_reserve(
    id: &str,
    entry_date: &str,
    connection_mp_ref: Option<String>,
    registrator_ref: &str,
    amount: f64,
) -> GeneralLedgerModel {
    let class = get_turnover_class(TURNOVER_CODE_RESERVE)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", TURNOVER_CODE_RESERVE));

    GeneralLedgerModel {
        id: id.to_string(),
        entry_date: entry_date.to_string(),
        layer: TurnoverLayer::Oper.as_str().to_string(),
        connection_mp_ref,
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: registrator_ref.to_string(),
        order_id: None,
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount,
        qty: None,
        turnover_code: TURNOVER_CODE_RESERVE.to_string(),
        resource_table: RESOURCE_TABLE_P913.to_string(),
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    }
}

fn to_general_ledger_entry_direct(
    id: &str,
    entry_date: &str,
    connection_mp_ref: Option<String>,
    registrator_ref: &str,
    amount: f64,
) -> GeneralLedgerModel {
    let class = get_turnover_class(TURNOVER_CODE_DIRECT)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", TURNOVER_CODE_DIRECT));

    GeneralLedgerModel {
        id: id.to_string(),
        entry_date: entry_date.to_string(),
        layer: TurnoverLayer::Oper.as_str().to_string(),
        connection_mp_ref,
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: registrator_ref.to_string(),
        order_id: None,
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount,
        qty: None,
        turnover_code: TURNOVER_CODE_DIRECT.to_string(),
        resource_table: RESOURCE_TABLE_P911.to_string(),
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    }
}

/// Округление до копейки (2 знака после запятой).
fn round_kopeyka(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn order_allocation_basis(order: &WbOrders) -> f64 {
    allocation_basis(order.line.price_with_disc, order.line.finished_price)
}

fn allocation_basis(price_with_disc: Option<f64>, finished_price: Option<f64>) -> f64 {
    price_with_disc.or(finished_price).unwrap_or(0.0)
}

fn allocate_costs(total_cost: f64, basis_flat: &[f64]) -> (Vec<f64>, f64, usize) {
    let total_basis: f64 = basis_flat.iter().copied().sum();
    let alloc_indices: Vec<usize> = if total_basis > f64::EPSILON {
        basis_flat
            .iter()
            .enumerate()
            .filter_map(|(i, &b)| if b > f64::EPSILON { Some(i) } else { None })
            .collect()
    } else {
        // Some WB order rows arrive from the orders endpoint without any price fields.
        // If every selected order has zero basis, keep attribution alive by splitting evenly.
        (0..basis_flat.len()).collect()
    };
    let n_with_price = alloc_indices.len();

    let mut alloc_flat: Vec<f64> = vec![0.0; basis_flat.len()];
    let mut accumulated = 0.0_f64;
    for (pos, &flat_i) in alloc_indices.iter().enumerate() {
        let basis = basis_flat[flat_i];
        let cost = if pos + 1 == n_with_price {
            round_kopeyka(total_cost - accumulated)
        } else {
            let raw_val = if total_basis > f64::EPSILON {
                total_cost * (basis / total_basis)
            } else if n_with_price > 0 {
                total_cost / n_with_price as f64
            } else {
                0.0
            };
            let rounded = round_kopeyka(raw_val);
            accumulated += rounded;
            rounded
        };
        alloc_flat[flat_i] = cost;
    }

    (alloc_flat, total_basis, n_with_price)
}

fn build_direct_p911_entries(
    document_id: Uuid,
    document: &WbAdvertDaily,
    linked_orders: &[WbAdvertLinkedOrdersByNm],
    general_ledger_ref: Option<&str>,
) -> Vec<crate::projections::p911_wb_advert_by_items::repository::Model> {
    let registrator_ref = document_id.to_string();
    let campaign_code = document.header.advert_id.to_string();
    let mut result = Vec::new();

    for (line, group) in document.lines.iter().zip(linked_orders.iter()) {
        let direct_amount = round_kopeyka((line.metrics.sum - group.wb_advert_sum).max(0.0));
        if direct_amount <= f64::EPSILON {
            continue;
        }

        let timestamp = now_str();
        result.push(
            crate::projections::p911_wb_advert_by_items::repository::Model {
                id: Uuid::new_v4().to_string(),
                connection_mp_ref: document.header.connection_id.clone(),
                entry_date: document.header.document_date.clone(),
                turnover_code: TURNOVER_CODE_DIRECT.to_string(),
                amount: direct_amount,
                nomenclature_ref: line.nomenclature_ref.clone(),
                wb_advert_campaign_code: campaign_code.clone(),
                registrator_type: REGISTRATOR_TYPE.to_string(),
                registrator_ref: registrator_ref.clone(),
                general_ledger_ref: general_ledger_ref.map(str::to_string),
                is_problem: line.nomenclature_ref.is_none(),
                created_at: timestamp.clone(),
                updated_at: timestamp,
            },
        );
    }

    let reserve_amount: f64 = linked_orders.iter().map(|group| group.wb_advert_sum).sum();
    let target_direct = round_kopeyka((document.totals.sum - reserve_amount).max(0.0));
    let actual_direct: f64 = result.iter().map(|entry| entry.amount).sum();
    let delta = round_kopeyka(target_direct - actual_direct);
    if delta.abs() > 0.01 {
        if let Some(last) = result.last_mut() {
            last.amount = round_kopeyka(last.amount + delta);
        } else if target_direct > f64::EPSILON {
            let timestamp = now_str();
            result.push(
                crate::projections::p911_wb_advert_by_items::repository::Model {
                    id: Uuid::new_v4().to_string(),
                    connection_mp_ref: document.header.connection_id.clone(),
                    entry_date: document.header.document_date.clone(),
                    turnover_code: TURNOVER_CODE_DIRECT.to_string(),
                    amount: target_direct,
                    nomenclature_ref: None,
                    wb_advert_campaign_code: campaign_code.clone(),
                    registrator_type: REGISTRATOR_TYPE.to_string(),
                    registrator_ref,
                    general_ledger_ref: general_ledger_ref.map(str::to_string),
                    is_problem: true,
                    created_at: timestamp.clone(),
                    updated_at: timestamp,
                },
            );
        }
    }

    result
}

/// Собирает связанные заказы по позициям рекламного отчёта.
///
/// Логика выборки per nm_id:
/// - кандидаты сортируются по уже начисленной атрибуции (ASC) из других
///   документов, чтобы приоритет получали заказы без накопленных начислений;
/// - берём первые N = wb_reported_orders кандидатов,
/// - если найдено меньше N — берём сколько есть,
/// - если найдено больше N — лишние показываются с is_allocated=false.
///
async fn build_linked_orders(
    document: &WbAdvertDaily,
    document_id: Uuid,
) -> Result<(i64, Vec<WbAdvertLinkedOrdersByNm>)> {
    // ── Phase 1: fetch all raw candidates ────────────────────────────────────
    let mut raw_per_nm: Vec<(i64, String, i64, f64, Vec<WbOrders>)> = Vec::new();

    for line in &document.lines {
        let raw = if line.metrics.orders > 0 {
            a015_wb_orders::repository::list_for_advert_attribution(
                line.nm_id,
                &document.header.connection_id,
                &document.header.document_date,
            )
            .await
            .unwrap_or_else(|err| {
                tracing::warn!(
                    "a026 linked orders lookup failed for nm_id={}: {}",
                    line.nm_id,
                    err
                );
                Vec::new()
            })
        } else {
            Vec::new()
        };
        raw_per_nm.push((
            line.nm_id,
            line.nm_name.clone(),
            line.metrics.orders,
            line.metrics.sum,
            raw,
        ));
    }

    // ── Phase 3: sort candidates by existing attribution (excluding self) ─────
    // Записи текущего документа исключаем — они будут удалены при перепроведении
    // и не должны искусственно повышать «накопленную нагрузку» его заказов.
    let self_ref = document_id.to_string();
    for (_, _, _, _, raw) in &mut raw_per_nm {
        let order_keys: Vec<String> = raw.iter().map(|o| o.header.document_no.clone()).collect();
        let existing_attr =
            crate::projections::p913_wb_advert_order_attr::repository::sum_reserve_by_order_keys(
                &order_keys,
                Some(&self_ref),
            )
            .await
            .unwrap_or_default();

        // Стабильная сортировка: при равной сумме сохраняется хронологический порядок.
        raw.sort_by(|a, b| {
            let sa = existing_attr
                .get(&a.header.document_no)
                .copied()
                .unwrap_or(0.0);
            let sb = existing_attr
                .get(&b.header.document_no)
                .copied()
                .unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // ── Phase 4: select first N per nm_id ────────────────────────────────────
    let mut per_line: Vec<(i64, String, i64, f64, Vec<WbOrders>, Vec<WbOrders>)> = Vec::new();
    for (nm_id, nm_name, wb_reported_orders, line_sum, raw) in raw_per_nm {
        let wb_reported = wb_reported_orders.max(0) as usize;
        let n_take = raw.len().min(wb_reported);
        let mut it = raw.into_iter();
        let selected: Vec<WbOrders> = it.by_ref().take(n_take).collect();
        let extras: Vec<WbOrders> = it.collect();
        per_line.push((
            nm_id,
            nm_name,
            wb_reported_orders,
            line_sum,
            selected,
            extras,
        ));
    }

    // ── Phase 5: allocate per a026 line ──────────────────────────────────────
    // Allocate each a026 line independently. If a line has no selected orders,
    // its amount is posted directly by nomenclature instead of being reserved.

    // Аллокация с last-residual округлением до копейки.
    // ── Phase 6: build groups ─────────────────────────────────────────────────
    let mut groups = Vec::new();
    let mut total_found = 0i64;

    for (nm_id, nm_name, wb_reported_orders, line_sum, selected, extras) in per_line {
        let n = selected.len();
        let group_basis: Vec<f64> = selected.iter().map(order_allocation_basis).collect();
        let (group_alloc, total_basis, n_with_price) = if n > 0 {
            allocate_costs(line_sum, &group_basis)
        } else {
            (Vec::new(), 0.0, 0)
        };

        let mut found_orders: Vec<WbAdvertFoundOrder> = selected
            .iter()
            .enumerate()
            .map(|(i, order)| {
                let basis = group_basis[i];
                let allocated_cost = group_alloc[i];
                let is_allocated = allocated_cost.abs() > f64::EPSILON;
                // allocation_ratio — доля цены этого заказа в ГЛОБАЛЬНОМ basis.
                let allocation_ratio = if is_allocated && total_basis > f64::EPSILON {
                    basis / total_basis
                } else if is_allocated && n_with_price > 0 {
                    1.0 / n_with_price as f64
                } else {
                    0.0
                };
                WbAdvertFoundOrder {
                    order_key: order.header.document_no.clone(),
                    order_id: Some(order.base.id.as_string()),
                    order_date: Some(order.state.order_dt.format("%Y-%m-%d").to_string()),
                    nomenclature_ref: order.nomenclature_ref.clone(),
                    finished_price: order.line.finished_price,
                    is_cancel: order.state.is_cancel,
                    allocation_basis: basis,
                    is_allocated,
                    allocation_ratio,
                    allocated_cost,
                }
            })
            .collect();

        // Лишние (за пределами N): видны в UI, расход не выделяется.
        for order in &extras {
            found_orders.push(WbAdvertFoundOrder {
                order_key: order.header.document_no.clone(),
                order_id: Some(order.base.id.as_string()),
                order_date: Some(order.state.order_dt.format("%Y-%m-%d").to_string()),
                nomenclature_ref: order.nomenclature_ref.clone(),
                finished_price: order.line.finished_price,
                is_cancel: order.state.is_cancel,
                allocation_basis: order_allocation_basis(order),
                is_allocated: false,
                allocation_ratio: 0.0,
                allocated_cost: 0.0,
            });
        }

        let wb_advert_sum: f64 = group_alloc.iter().copied().sum();
        total_found += n as i64;

        groups.push(WbAdvertLinkedOrdersByNm {
            nm_id,
            nm_name,
            wb_reported_orders,
            wb_advert_sum,
            found_orders,
        });
    }

    Ok((total_found, groups))
}

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    let (total_found, linked_orders) = build_linked_orders(&document, id).await?;
    document.linked_orders_count = total_found;
    document.has_linked_orders = total_found > 0;
    document.linked_orders = linked_orders;

    document.is_posted = true;
    document.base.metadata.is_posted = true;
    document.before_write();
    repository::upsert_document(&document).await?;

    tracing::info!(
        "a026 post_document: linked orders found={} for document_id={}",
        total_found,
        id
    );

    let registrator_ref = id.to_string();

    // GL проводка advert_clicks_order_accrual (Phase 1 p913): Д9601/К7609.
    let reserve_amount: f64 = document
        .linked_orders
        .iter()
        .map(|group| group.wb_advert_sum)
        .sum();
    let direct_amount = round_kopeyka((document.totals.sum - reserve_amount).max(0.0));

    let gl_reserve_entry = if reserve_amount.abs() > f64::EPSILON {
        let gl_reserve_ref = Uuid::new_v4().to_string();
        Some(to_general_ledger_entry_reserve(
            &gl_reserve_ref,
            &document.header.document_date,
            Some(document.header.connection_id.clone()),
            &registrator_ref,
            reserve_amount,
        ))
    } else {
        None
    };
    let gl_direct_entry = if direct_amount.abs() > f64::EPSILON {
        let gl_direct_ref = Uuid::new_v4().to_string();
        Some(to_general_ledger_entry_direct(
            &gl_direct_ref,
            &document.header.document_date,
            Some(document.header.connection_id.clone()),
            &registrator_ref,
            direct_amount,
        ))
    } else {
        None
    };

    // p913 reserve-строки per-order. Привязываем каждую к id GL-проводки,
    // чтобы general_ledger_ref всегда был валидным.
    let p913_entries =
        crate::projections::p913_wb_advert_order_attr::service::build_reserve_entries(
            id,
            &document,
            gl_reserve_entry.as_ref().map(|entry| entry.id.as_str()),
        );
    let p911_entries = build_direct_p911_entries(
        id,
        &document,
        &document.linked_orders,
        gl_direct_entry.as_ref().map(|entry| entry.id.as_str()),
    );

    let db = get_connection();
    let txn = db.begin().await?;

    crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;
    crate::projections::p913_wb_advert_order_attr::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;
    crate::projections::p911_wb_advert_by_items::repository::delete_by_registrator_ref_with_conn(
        &txn,
        &registrator_ref,
    )
    .await?;
    crate::projections::p911_wb_advert_by_items::repository::delete_by_registrator_ref_with_conn(
        &txn,
        &format!("a026:{registrator_ref}"),
    )
    .await?;

    if let Some(entry) = &gl_reserve_entry {
        crate::projections::general_ledger::repository::save_entry_with_conn(&txn, entry).await?;
    }
    if let Some(entry) = &gl_direct_entry {
        crate::projections::general_ledger::repository::save_entry_with_conn(&txn, entry).await?;
    }
    for entry in &p913_entries {
        crate::projections::p913_wb_advert_order_attr::repository::save_entry_with_conn(
            &txn, entry,
        )
        .await?;
    }
    for entry in &p911_entries {
        crate::projections::p911_wb_advert_by_items::repository::save_entry_with_conn(&txn, entry)
            .await?;
    }

    txn.commit().await?;

    Ok(())
}

pub async fn unpost_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    document.is_posted = false;
    document.base.metadata.is_posted = false;
    document.has_linked_orders = false;
    document.linked_orders_count = 0;
    document.linked_orders.clear();
    document.before_write();
    repository::upsert_document(&document).await?;

    let registrator_ref = id.to_string();

    let db = get_connection();
    let txn = db.begin().await?;

    crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;
    crate::projections::p913_wb_advert_order_attr::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;
    crate::projections::p911_wb_advert_by_items::repository::delete_by_registrator_ref_with_conn(
        &txn,
        &registrator_ref,
    )
    .await?;
    crate::projections::p911_wb_advert_by_items::repository::delete_by_registrator_ref_with_conn(
        &txn,
        &format!("a026:{registrator_ref}"),
    )
    .await?;

    txn.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{allocate_costs, allocation_basis};

    #[test]
    fn allocation_basis_prefers_price_with_disc() {
        assert_eq!(allocation_basis(Some(80.0), Some(100.0)), 80.0);
        assert_eq!(allocation_basis(None, Some(100.0)), 100.0);
        assert_eq!(allocation_basis(None, None), 0.0);
    }

    #[test]
    fn allocate_costs_splits_evenly_when_all_basis_is_zero() {
        let (allocated, total_basis, count) = allocate_costs(11940.4, &[0.0, 0.0]);

        assert_eq!(total_basis, 0.0);
        assert_eq!(count, 2);
        assert_eq!(allocated, vec![5970.2, 5970.2]);
    }

    #[test]
    fn allocate_costs_uses_positive_basis_when_available() {
        let (allocated, total_basis, count) = allocate_costs(100.0, &[0.0, 30.0, 70.0]);

        assert_eq!(total_basis, 100.0);
        assert_eq!(count, 2);
        assert_eq!(allocated, vec![0.0, 30.0, 70.0]);
    }
}
