use anyhow::Result;
use chrono::Utc;
use contracts::domain::a026_wb_advert_daily::aggregate::{WbAdvertDaily, WbAdvertDailyLine};
use contracts::shared::analytics::TurnoverLayer;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use super::repository;
use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::projections::p911_wb_advert_by_items::repository::Model as WbAdvertByItemModel;
use crate::shared::analytics::normalization::normalize_positive;
use crate::shared::data::db::get_connection;

const REGISTRATOR_TYPE: &str = "a026_wb_advert_daily";
const TURNOVER_CODE: &str = "advertising_allocated";

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
        resource_table: REGISTRATOR_TYPE.to_string(),
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

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    document.is_posted = true;
    document.base.metadata.is_posted = true;
    document.before_write();
    repository::upsert_document(&document).await?;

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
}
