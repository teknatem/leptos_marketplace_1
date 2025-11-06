use super::repository;
use anyhow::Result;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use uuid::Uuid;

pub async fn store_document_with_raw(mut document: WbSales, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "WB",
        "WB_Sales",
        &document.header.document_no,
        raw_json,
        document.source_meta.fetched_at,
    )
    .await?;

    document.source_meta.raw_payload_ref = raw_ref;
    document
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    document.before_write();

    let id = repository::upsert_document(&document).await?;
    
    // Проецируем в Sales Register с реальным UUID из БД
    if let Err(e) = crate::projections::p900_mp_sales_register::service::project_wb_sales(&document, id).await {
        tracing::error!("Failed to project WB Sales document to Sales Register: {}", e);
    }
    
    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSales>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<WbSales>> {
    repository::get_by_document_no(document_no).await
}

pub async fn list_all() -> Result<Vec<WbSales>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

