use super::repository;
use anyhow::Result;
use chrono::NaiveDate;
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

    tracing::info!("Successfully saved WB Sales document with id: {}", id);

    // Проводим документ если is_posted = true
    if document.is_posted {
        if let Err(e) = super::posting::post_document(id).await {
            tracing::error!("Failed to post WB Sales document: {}", e);
            // Не останавливаем выполнение, т.к. документ уже сохранен
        }
    } else {
        // Если is_posted = false, удаляем проекции (если были)
        if let Err(e) = crate::projections::p900_mp_sales_register::repository::delete_by_registrator(&id.to_string()).await {
            tracing::error!("Failed to delete projections for WB Sales document: {}", e);
        }
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

pub async fn list_by_date_range(
    date_from: Option<NaiveDate>,
    date_to: Option<NaiveDate>,
) -> Result<Vec<WbSales>> {
    repository::list_by_date_range(date_from, date_to).await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

