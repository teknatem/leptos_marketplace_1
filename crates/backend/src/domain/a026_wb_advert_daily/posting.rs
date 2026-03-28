use anyhow::Result;
use chrono::Utc;
use contracts::shared::analytics::TurnoverLayer;
use uuid::Uuid;

use super::repository;
use crate::projections::general_ledger::repository::Model as GeneralLedgerModel;
use crate::shared::analytics::normalization::normalize_positive;
use crate::shared::analytics::turnover_registry::get_turnover_class;

const REGISTRATOR_TYPE: &str = "a026_wb_advert_daily";
const TURNOVER_CODE: &str = "advertising_allocated";

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn to_general_ledger_entry(
    id: &str,
    _posting_id: &str,
    entry_date: &str,
    cabinet_mp: Option<String>,
    registrator_ref: &str,
    amount: f64,
) -> GeneralLedgerModel {
    let class = get_turnover_class(TURNOVER_CODE)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", TURNOVER_CODE));

    GeneralLedgerModel {
        id: id.to_string(),
        entry_date: entry_date.to_string(),
        layer: TurnoverLayer::Oper.as_str().to_string(),
        cabinet_mp,
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: registrator_ref.to_string(),
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

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    document.is_posted = true;
    document.base.metadata.is_posted = true;
    document.before_write();
    repository::upsert_document(&document).await?;

    let registrator_ref = format!("a026:{}", id);
    crate::projections::general_ledger::service::remove_by_registrator_ref(&registrator_ref)
        .await?;

    let mut total_amount = 0.0;
    for line in &document.lines {
        let Some(amount) = normalize_positive(Some(line.metrics.sum)) else {
            continue;
        };

        total_amount += amount;
    }

    if total_amount.abs() > f64::EPSILON {
        let general_ledger_ref = Uuid::new_v4().to_string();
        let general_ledger_entry = to_general_ledger_entry(
            &general_ledger_ref,
            "",
            &document.header.document_date,
            Some(document.header.connection_id.clone()),
            &registrator_ref,
            total_amount,
        );
        crate::projections::general_ledger::service::save_entries(&[general_ledger_entry]).await?;
    }

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

    let registrator_ref = format!("a026:{}", id);
    crate::projections::general_ledger::service::remove_by_registrator_ref(&registrator_ref)
        .await?;

    Ok(())
}
