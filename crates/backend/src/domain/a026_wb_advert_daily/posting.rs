use anyhow::Result;
use chrono::Utc;
use contracts::shared::analytics::{TurnoverLayer, ValueKind};
use uuid::Uuid;

use super::repository;
use crate::projections::general_ledger::repository::Model as GeneralLedgerModel;
use crate::projections::p911_wb_advert_by_items::repository::Model as P911Model;
use crate::shared::analytics::normalization::normalize_positive;
use crate::shared::analytics::turnover_registry::get_turnover_class;

const REGISTRATOR_TYPE: &str = "a026_wb_advert_daily";
const TURNOVER_CODE: &str = "advertising_allocated";

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn to_general_ledger_entry(
    id: &str,
    posting_id: &str,
    entry_date: &str,
    registrator_ref: &str,
    amount: f64,
) -> GeneralLedgerModel {
    let class = get_turnover_class(TURNOVER_CODE)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", TURNOVER_CODE));

    GeneralLedgerModel {
        id: id.to_string(),
        posting_id: posting_id.to_string(),
        entry_date: entry_date.to_string(),
        layer: TurnoverLayer::Oper.as_str().to_string(),
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: registrator_ref.to_string(),
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount,
        qty: None,
        turnover_code: TURNOVER_CODE.to_string(),
        detail_kind: "p911_wb_advert_by_items".to_string(),
        detail_id: id.to_string(),
        resource_name: TURNOVER_CODE.to_string(),
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
    crate::projections::p911_wb_advert_by_items::service::remove_by_registrator_ref(
        &registrator_ref,
    )
    .await?;
    crate::projections::general_ledger::service::remove_by_registrator_ref(&registrator_ref)
        .await?;

    let posting_id = Uuid::new_v4().to_string();
    let general_ledger_ref = Uuid::new_v4().to_string();
    let now = now_str();

    let mut total_amount = 0.0;
    for line in &document.lines {
        let Some(nomenclature_ref) = line
            .nomenclature_ref
            .clone()
            .filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        let Some(amount) = normalize_positive(Some(line.metrics.sum)) else {
            continue;
        };

        total_amount += amount;

        let entry = P911Model {
            id: format!(
                "{}:{}:{}:{}",
                document.header.connection_id, id, line.nm_id, TURNOVER_CODE
            ),
            connection_mp_ref: document.header.connection_id.clone(),
            entry_date: document.header.document_date.clone(),
            layer: TurnoverLayer::Oper.as_str().to_string(),
            turnover_code: TURNOVER_CODE.to_string(),
            value_kind: ValueKind::Money.as_str().to_string(),
            agg_kind: "sum".to_string(),
            amount,
            nomenclature_ref: Some(nomenclature_ref),
            registrator_type: REGISTRATOR_TYPE.to_string(),
            registrator_ref: registrator_ref.clone(),
            general_ledger_ref: Some(general_ledger_ref.clone()),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        crate::projections::p911_wb_advert_by_items::service::save_entry(&entry).await?;
    }

    if total_amount.abs() > f64::EPSILON {
        let general_ledger_entry = to_general_ledger_entry(
            &general_ledger_ref,
            &posting_id,
            &document.header.document_date,
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
    crate::projections::p911_wb_advert_by_items::service::remove_by_registrator_ref(
        &registrator_ref,
    )
    .await?;
    crate::projections::general_ledger::service::remove_by_registrator_ref(&registrator_ref)
        .await?;

    Ok(())
}
