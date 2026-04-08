use super::repository;
use anyhow::Result;
use uuid::Uuid;

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

    if should_persist_document {
        document.before_write();
        repository::upsert_document(&document).await?;
    }

    let prod_item_cost_total =
        super::service::resolve_prod_item_cost_total_cached(&document, cache).await?;

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

    crate::projections::p900_mp_sales_register::service::project_wb_sales(&document, id).await?;
    crate::projections::p904_sales_data::service::project_wb_sales(&document, id).await?;
    crate::projections::p909_mp_order_line_turnovers::service::project_wb_sales_fresh(
        &document,
        id,
        &registrator_ref,
        prod_item_cost_total,
    )
    .await?;

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

    Ok(())
}
