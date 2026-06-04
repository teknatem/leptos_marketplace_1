//! Проведение документа реализации YM в General Ledger на слое `ybuh`.
//!
//! Один документ = один кабинет × одна дата. Проводка агрегирует строки SKU в
//! контрольные итоги день×кабинет:
//!   - продажи  → `customer_revenue`        (Дт7609/Кт9001, сумма +)
//!   - возвраты → `customer_revenue_storno` (Дт7609/Кт9001, сумма −)
//!
//! Слой `ybuh` — независимый источник для сверки с `fina` (p907) по тем же
//! оборотам. p914-зеркало НЕ строим (ybuh ≠ fina). Идемпотентно: перед записью
//! удаляем прежние GL-проводки документа.

use anyhow::Result;
use chrono::Utc;
use contracts::shared::analytics::TurnoverLayer;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use super::repository;
use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::shared::data::db::get_connection;

const REGISTRATOR_TYPE: &str = "a034_ym_realization";
const TURNOVER_CODE_REVENUE: &str = "customer_revenue";
const TURNOVER_CODE_REVENUE_STORNO: &str = "customer_revenue_storno";

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn round_kopeyka(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn is_significant(amount: f64) -> bool {
    amount.abs() > 0.005
}

fn gl_entry(
    turnover_code: &str,
    entry_date: &str,
    connection_mp_ref: Option<String>,
    registrator_ref: &str,
    amount: f64,
    qty: f64,
) -> GeneralLedgerModel {
    let class = get_turnover_class(turnover_code)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", turnover_code));

    GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        entry_date: entry_date.to_string(),
        layer: TurnoverLayer::Ybuh.as_str().to_string(),
        // Субъект учёта проставится при переработке ybuh-источника (Фаза 2+).
        entity: None,
        connection_mp_ref,
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: registrator_ref.to_string(),
        order_id: None,
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount,
        qty: Some(qty),
        turnover_code: turnover_code.to_string(),
        // У слоя ybuh нет проекции-зеркала: drilldown только по common-измерениям.
        resource_table: String::new(),
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    }
}

/// Собирает GL-проводки документа (выручка/сторно). Public для юнит-тестов.
pub fn build_general_ledger_entries(document: &super::DocumentForPosting) -> Vec<GeneralLedgerModel> {
    let mut entries = Vec::new();
    let entry_date = &document.document_date;
    let connection = Some(document.connection_id.clone());
    let registrator_ref = &document.id;

    let sales = round_kopeyka(document.sales_revenue);
    if is_significant(sales) {
        entries.push(gl_entry(
            TURNOVER_CODE_REVENUE,
            entry_date,
            connection.clone(),
            registrator_ref,
            sales,
            document.sales_qty,
        ));
    }

    let returns = round_kopeyka(document.return_revenue);
    if is_significant(returns) {
        entries.push(gl_entry(
            TURNOVER_CODE_REVENUE_STORNO,
            entry_date,
            connection,
            registrator_ref,
            -returns,
            // Кол-во возвратов — отрицательным, чтобы нетто-кол-во = продажи − возвраты.
            -document.return_qty,
        ));
    }

    entries
}

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    document.is_posted = true;
    document.base.metadata.is_posted = true;
    document.before_write();

    let registrator_ref = id.to_string();
    let entries = build_general_ledger_entries(&super::DocumentForPosting {
        id: registrator_ref.clone(),
        document_date: document.header.document_date.clone(),
        connection_id: document.header.connection_id.clone(),
        sales_revenue: document.totals.sales_revenue,
        return_revenue: document.totals.return_revenue,
        sales_qty: document.totals.sales_qty,
        return_qty: document.totals.return_qty,
    });

    let db = get_connection();
    let txn = db.begin().await?;

    if !repository::exists_with_conn(&txn, &registrator_ref).await? {
        txn.rollback().await?;
        anyhow::bail!(
            "Document {} was removed during posting preparation; GL was not written",
            id
        );
    }

    repository::update_document_with_conn(&txn, &document).await?;

    crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;

    for entry in &entries {
        crate::projections::general_ledger::repository::save_entry_with_conn(&txn, entry).await?;
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

    let registrator_ref = id.to_string();
    let db = get_connection();
    let txn = db.begin().await?;
    repository::update_document_with_conn(&txn, &document).await?;
    crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;
    txn.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(sales: f64, returns: f64) -> super::super::DocumentForPosting {
        super::super::DocumentForPosting {
            id: "doc-1".to_string(),
            document_date: "2026-05-10".to_string(),
            connection_id: "conn-1".to_string(),
            sales_revenue: sales,
            return_revenue: returns,
            sales_qty: 4.0,
            return_qty: 1.0,
        }
    }

    #[test]
    fn sales_and_returns_produce_two_ybuh_entries() {
        let entries = build_general_ledger_entries(&doc(1000.0, 150.0));
        assert_eq!(entries.len(), 2);

        let revenue = entries
            .iter()
            .find(|e| e.turnover_code == "customer_revenue")
            .unwrap();
        assert_eq!(revenue.layer, "ybuh");
        assert_eq!(revenue.amount, 1000.0);
        assert_eq!(revenue.qty, Some(4.0));
        assert_eq!(revenue.debit_account, "7609");
        assert_eq!(revenue.credit_account, "9001");
        assert_eq!(revenue.entry_date, "2026-05-10");

        let storno = entries
            .iter()
            .find(|e| e.turnover_code == "customer_revenue_storno")
            .unwrap();
        assert_eq!(storno.layer, "ybuh");
        assert_eq!(storno.amount, -150.0);
        assert_eq!(storno.qty, Some(-1.0));
    }

    #[test]
    fn zero_returns_produce_single_entry() {
        let entries = build_general_ledger_entries(&doc(500.0, 0.0));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].turnover_code, "customer_revenue");
        assert_eq!(entries[0].amount, 500.0);
    }

    #[test]
    fn empty_document_produces_no_entries() {
        let entries = build_general_ledger_entries(&doc(0.0, 0.0));
        assert!(entries.is_empty());
    }
}
