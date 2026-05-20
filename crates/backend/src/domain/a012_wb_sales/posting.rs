use super::repository;
use anyhow::Result;
use chrono::Utc;
use contracts::shared::analytics::TurnoverLayer;
use uuid::Uuid;

use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;

const REGISTRATOR_TYPE: &str = "a012_wb_sales";
const TURNOVER_CODE_EXPENSE: &str = "advert_clicks_order_expense";
const RESOURCE_TABLE_P913: &str = "p913_wb_advert_order_attr";

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn to_gl_advert_expense(
    gl_id: &str,
    entry_date: &str,
    connection_mp_ref: Option<String>,
    registrator_ref: &str,
    amount: f64,
) -> GeneralLedgerModel {
    let class = get_turnover_class(TURNOVER_CODE_EXPENSE)
        .unwrap_or_else(|| panic!("Missing turnover class for {}", TURNOVER_CODE_EXPENSE));

    GeneralLedgerModel {
        id: gl_id.to_string(),
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
        turnover_code: TURNOVER_CODE_EXPENSE.to_string(),
        resource_table: RESOURCE_TABLE_P913.to_string(),
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    }
}

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut cache = super::service::PostingPreparationCache::default();
    post_document_with_cache(id, &mut cache).await
}

pub async fn post_document_with_cache(
    id: Uuid,
    cache: &mut super::service::PostingPreparationCache,
) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    let prepare_changed =
        super::service::prepare_document_for_posting_cached(&mut document, cache).await?;

    let next_is_customer_return = document.state.event_type.eq_ignore_ascii_case("return")
        || document.line.finished_price.unwrap_or(0.0) < 0.0;
    let mut should_persist_document =
        prepare_changed || !document.is_posted || !document.base.metadata.is_posted;

    if document.is_customer_return != next_is_customer_return {
        document.is_customer_return = next_is_customer_return;
        should_persist_document = true;
    }
    if !document.is_posted {
        document.is_posted = true;
        should_persist_document = true;
    }
    if !document.base.metadata.is_posted {
        document.base.metadata.is_posted = true;
        should_persist_document = true;
    }

    let prod_cost_resolution = super::service::resolve_prod_cost_cached(&document, cache).await?;
    should_persist_document |=
        super::service::apply_prod_cost_diagnostics(&mut document, &prod_cost_resolution);

    if should_persist_document {
        document.before_write();
        repository::upsert_document(&document).await?;
    }

    let prod_item_cost_total = prod_cost_resolution.total;

    let registrator_ref = id.to_string();
    let p909_registrator_ref = format!("a012:{id}");

    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&registrator_ref)
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&registrator_ref)
        .await?;
    crate::projections::p909_mp_order_line_turnovers::service::remove_by_registrator_ref(
        &p909_registrator_ref,
    )
    .await?;
    crate::general_ledger::service::remove_by_registrator("a012_wb_sales", &registrator_ref)
        .await?;
    // p913 expense строки используют "a012:{id}" как registrator_ref.
    crate::projections::p913_wb_advert_order_attr::repository::delete_by_registrator(
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;

    crate::projections::p900_mp_sales_register::service::project_wb_sales(&document, id).await?;
    crate::projections::p904_sales_data::service::project_wb_sales(&document, id).await?;
    crate::projections::p909_mp_order_line_turnovers::service::project_wb_sales_fresh(
        &document,
        id,
        &registrator_ref,
        prod_item_cost_total,
    )
    .await?;

    // Phase 2 p913: создаём expense-строки только при реализации (не возврат).
    if !document.is_customer_return {
        let srid = &document.header.document_no;
        let reserve_rows =
            crate::projections::p913_wb_advert_order_attr::repository::list_by_order_key_and_turnover(
                srid, "advert_clicks_order_accrual",
            )
            .await?;
        if !reserve_rows.is_empty() {
            let total_amount: f64 = reserve_rows.iter().map(|r| r.amount).sum();
            let entry_date = document.state.sale_dt.format("%Y-%m-%d").to_string();
            let gl_id = Uuid::new_v4().to_string();
            let gl_entry = to_gl_advert_expense(
                &gl_id,
                &entry_date,
                Some(document.header.connection_id.clone()),
                &registrator_ref,
                total_amount,
            );
            crate::projections::general_ledger::repository::save_entry(&gl_entry).await?;
            let sale_finished_price = document.line.finished_price.unwrap_or(0.0);
            for entry in
                crate::projections::p913_wb_advert_order_attr::service::build_expense_entries(
                    id,
                    srid,
                    &reserve_rows,
                    sale_finished_price,
                    &gl_id,
                )
            {
                crate::projections::p913_wb_advert_order_attr::repository::save_entry(&entry)
                    .await?;
            }
        }
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

    let registrator_ref = id.to_string();
    let p909_registrator_ref = format!("a012:{id}");

    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&registrator_ref)
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&registrator_ref)
        .await?;
    crate::projections::p909_mp_order_line_turnovers::service::remove_by_registrator_ref(
        &p909_registrator_ref,
    )
    .await?;
    crate::general_ledger::service::remove_by_registrator("a012_wb_sales", &registrator_ref)
        .await?;
    crate::projections::p913_wb_advert_order_attr::repository::delete_by_registrator(
        REGISTRATOR_TYPE,
        &registrator_ref,
    )
    .await?;

    Ok(())
}
