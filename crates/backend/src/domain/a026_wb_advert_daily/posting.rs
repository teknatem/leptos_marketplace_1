use anyhow::Result;
use chrono::Utc;
use contracts::domain::a026_wb_advert_daily::aggregate::{
    WbAdvertDaily, WbAdvertDailyLine, WbAdvertFoundOrder, WbAdvertLinkedOrdersByNm,
};
use contracts::shared::analytics::TurnoverLayer;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use super::repository;
use crate::domain::a015_wb_orders;
use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::projections::p911_wb_advert_by_items::repository::Model as WbAdvertByItemModel;
use crate::shared::analytics::normalization::normalize_positive;
use crate::shared::data::db::get_connection;

const REGISTRATOR_TYPE: &str = "a026_wb_advert_daily";
const TURNOVER_CODE: &str = "advertising_allocated";
const RESOURCE_TABLE: &str = "p911_wb_advert_by_items";

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn projection_registrator_ref(id: Uuid) -> String {
    format!("a026:{id}")
}

fn to_general_ledger_entry(
    id: &str,
    _posting_id: &str,
    entry_date: &str,
    connection_mp_ref: Option<String>,
    registrator_ref: &str,
    amount: f64,
) -> GeneralLedgerModel {
    let class = get_turnover_class(TURNOVER_CODE)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", TURNOVER_CODE));

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
        turnover_code: TURNOVER_CODE.to_string(),
        resource_table: RESOURCE_TABLE.to_string(),
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    }
}

fn to_projection_entry(
    document_id: Uuid,
    document: &WbAdvertDaily,
    line: &WbAdvertDailyLine,
    general_ledger_ref: Option<String>,
) -> Option<WbAdvertByItemModel> {
    let Some(amount) = normalize_positive(Some(line.metrics.sum)) else {
        return None;
    };

    let class = get_turnover_class(TURNOVER_CODE)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", TURNOVER_CODE));
    let timestamp = now_str();

    Some(WbAdvertByItemModel {
        id: format!(
            "{}:{}:{}:{}",
            document.header.connection_id, document_id, line.nm_id, TURNOVER_CODE
        ),
        connection_mp_ref: document.header.connection_id.clone(),
        entry_date: document.header.document_date.clone(),
        layer: TurnoverLayer::Oper.as_str().to_string(),
        turnover_code: TURNOVER_CODE.to_string(),
        value_kind: class.value_kind.as_str().to_string(),
        agg_kind: class.agg_kind.as_str().to_string(),
        amount,
        nomenclature_ref: line.nomenclature_ref.clone(),
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: projection_registrator_ref(document_id),
        general_ledger_ref,
        is_problem: line
            .nomenclature_ref
            .as_ref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    })
}

fn build_projection_entries(
    document_id: Uuid,
    document: &WbAdvertDaily,
    general_ledger_ref: Option<String>,
) -> Vec<WbAdvertByItemModel> {
    document
        .lines
        .iter()
        .filter_map(|line| {
            to_projection_entry(document_id, document, line, general_ledger_ref.clone())
        })
        .collect()
}

/// Находит и возвращает группы связанных заказов по позициям рекламного отчёта.
/// Возвращает (всего_найдено_заказов, группы_по_nm_id).
async fn build_linked_orders(
    document: &WbAdvertDaily,
) -> Result<(i64, Vec<WbAdvertLinkedOrdersByNm>)> {
    let mut groups = Vec::with_capacity(document.lines.len());
    let mut total_found: i64 = 0;

    for line in &document.lines {
        // Ищем заказы только по позициям, по которым WB зарегистрировал заказы.
        // metrics.orders > 0 эквивалентно sum_price > 0 — это позиции с
        // фактической выручкой от рекламы, остальные nm_id рекламировались, но
        // заказов не принесли и привязка к ним смысла не имеет.
        if line.metrics.orders <= 0 {
            continue;
        }

        let raw_candidates = a015_wb_orders::repository::list_for_advert_attribution(
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
        });

        // Первые N (по хронологии ASC) участвуют в аллокации расхода.
        // Остальные показываются в UI с is_allocated=false, allocated_cost=0.
        let wb_reported = line.metrics.orders.max(0) as usize;
        let selected = &raw_candidates[..raw_candidates.len().min(wb_reported)];

        // basis считаем только по выбранным N.
        let basis_selected: Vec<f64> = selected
            .iter()
            .map(|order| {
                order
                    .line
                    .finished_price
                    .or(order.line.price_with_disc)
                    .unwrap_or(0.0)
            })
            .collect();
        let total_basis: f64 = basis_selected.iter().copied().sum();

        let wb_advert_sum = line.metrics.sum;
        let n_selected = selected.len();

        // Аллокация с last-residual округлением до копейки.
        let mut allocated_costs: Vec<f64> = Vec::with_capacity(n_selected);
        let mut accumulated = 0.0_f64;
        for (idx, &basis) in basis_selected.iter().enumerate() {
            let cost = if idx + 1 == n_selected {
                round_kopeyka(wb_advert_sum - accumulated)
            } else {
                let raw = if total_basis > f64::EPSILON {
                    wb_advert_sum * (basis / total_basis)
                } else if n_selected > 0 {
                    wb_advert_sum / n_selected as f64
                } else {
                    0.0
                };
                let rounded = round_kopeyka(raw);
                accumulated += rounded;
                rounded
            };
            allocated_costs.push(cost);
        }

        // Строим итоговый список: сначала выбранные (с расходом), затем лишние.
        let mut found_orders: Vec<WbAdvertFoundOrder> = selected
            .iter()
            .zip(basis_selected.iter())
            .zip(allocated_costs.iter())
            .map(|((order, &basis), &allocated_cost)| {
                let allocation_ratio = if total_basis > f64::EPSILON {
                    basis / total_basis
                } else if n_selected > 0 {
                    1.0 / n_selected as f64
                } else {
                    0.0
                };
                WbAdvertFoundOrder {
                    order_key: order.header.document_no.clone(),
                    nomenclature_ref: order.nomenclature_ref.clone(),
                    finished_price: order.line.finished_price,
                    is_cancel: order.state.is_cancel,
                    is_allocated: true,
                    allocation_ratio,
                    allocated_cost,
                }
            })
            .collect();

        // Лишние кандидаты (за пределами N): видны в UI, расход = 0.
        for order in raw_candidates.iter().skip(wb_reported) {
            found_orders.push(WbAdvertFoundOrder {
                order_key: order.header.document_no.clone(),
                nomenclature_ref: order.nomenclature_ref.clone(),
                finished_price: order.line.finished_price,
                is_cancel: order.state.is_cancel,
                is_allocated: false,
                allocation_ratio: 0.0,
                allocated_cost: 0.0,
            });
        }

        total_found += n_selected as i64;

        groups.push(WbAdvertLinkedOrdersByNm {
            nm_id: line.nm_id,
            nm_name: line.nm_name.clone(),
            wb_reported_orders: line.metrics.orders,
            wb_advert_sum,
            found_orders,
        });
    }

    Ok((total_found, groups))
}

/// Округление до копейки (2 знака после запятой), bankers-friendly через f64.
fn round_kopeyka(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    let (total_found, linked_orders) = build_linked_orders(&document).await?;
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
    let projection_ref = projection_registrator_ref(id);

    let mut total_amount = 0.0;
    for line in &document.lines {
        let Some(amount) = normalize_positive(Some(line.metrics.sum)) else {
            continue;
        };

        total_amount += amount;
    }

    let general_ledger_entry = if total_amount.abs() > f64::EPSILON {
        let general_ledger_ref = Uuid::new_v4().to_string();
        Some(to_general_ledger_entry(
            &general_ledger_ref,
            "",
            &document.header.document_date,
            Some(document.header.connection_id.clone()),
            &registrator_ref,
            total_amount,
        ))
    } else {
        None
    };

    let projection_entries = build_projection_entries(
        id,
        &document,
        general_ledger_entry.as_ref().map(|entry| entry.id.clone()),
    );

    let db = get_connection();
    let txn = db.begin().await?;

    crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;
    crate::projections::p911_wb_advert_by_items::repository::delete_by_registrator_ref_with_conn(
        &txn,
        &projection_ref,
    )
    .await?;

    if let Some(entry) = &general_ledger_entry {
        crate::projections::general_ledger::repository::save_entry_with_conn(&txn, entry).await?;
    }

    for entry in &projection_entries {
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
    let projection_ref = projection_registrator_ref(id);

    let db = get_connection();
    let txn = db.begin().await?;

    crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;
    crate::projections::p911_wb_advert_by_items::repository::delete_by_registrator_ref_with_conn(
        &txn,
        &projection_ref,
    )
    .await?;

    txn.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_projection_entries;
    use contracts::domain::a026_wb_advert_daily::aggregate::{
        WbAdvertDaily, WbAdvertDailyHeader, WbAdvertDailyLine, WbAdvertDailyMetrics,
        WbAdvertDailySourceMeta,
    };
    use uuid::Uuid;

    fn sample_document(lines: Vec<WbAdvertDailyLine>) -> WbAdvertDaily {
        WbAdvertDaily::new_for_insert(
            WbAdvertDailyHeader {
                document_no: "WB-ADV-TEST".to_string(),
                document_date: "2026-02-12".to_string(),
                advert_id: 999,
                connection_id: "conn-1".to_string(),
                organization_id: "org-1".to_string(),
                marketplace_id: "wb".to_string(),
            },
            WbAdvertDailyMetrics::default(),
            WbAdvertDailyMetrics::default(),
            lines,
            WbAdvertDailySourceMeta {
                source: "test".to_string(),
                fetched_at: "2026-02-12T00:00:00Z".to_string(),
            },
        )
    }

    #[test]
    fn projection_builder_keeps_problem_rows_in_sum() {
        let document = sample_document(vec![
            WbAdvertDailyLine {
                nm_id: 1,
                nm_name: "Matched".to_string(),
                nomenclature_ref: Some("nom-1".to_string()),
                advert_ids: vec![1],
                app_types: vec![32],
                placements: vec!["search".to_string()],
                metrics: WbAdvertDailyMetrics {
                    sum: 10.0,
                    ..WbAdvertDailyMetrics::default()
                },
            },
            WbAdvertDailyLine {
                nm_id: 2,
                nm_name: "Problem".to_string(),
                nomenclature_ref: None,
                advert_ids: vec![2],
                app_types: vec![64],
                placements: vec![],
                metrics: WbAdvertDailyMetrics {
                    sum: 5.0,
                    ..WbAdvertDailyMetrics::default()
                },
            },
        ]);

        let rows = build_projection_entries(Uuid::nil(), &document, Some("gl-1".to_string()));

        assert_eq!(rows.len(), 2);
        assert!(!rows[0].is_problem);
        assert!(rows[1].is_problem);
        assert_eq!(rows.iter().map(|row| row.amount).sum::<f64>(), 15.0);
        assert!(rows
            .iter()
            .all(|row| row.general_ledger_ref.as_deref() == Some("gl-1")));
    }

    #[test]
    fn general_ledger_entry_points_to_p911_projection() {
        let entry = super::to_general_ledger_entry(
            "gl-1",
            "",
            "2026-02-12",
            Some("conn-1".to_string()),
            "doc-1",
            15.0,
        );

        assert_eq!(entry.resource_table, super::RESOURCE_TABLE);
        assert_eq!(entry.resource_field, "amount");
        assert_eq!(entry.resource_sign, 1);
    }
}
