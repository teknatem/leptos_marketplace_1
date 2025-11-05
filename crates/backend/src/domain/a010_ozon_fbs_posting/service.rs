use super::repository;
use anyhow::Result;
use contracts::domain::a010_ozon_fbs_posting::aggregate::OzonFbsPosting;
use contracts::domain::common::AggregateId;
use uuid::Uuid;

/// Сохранить документ с сырым JSON
/// Идемпотентная операция - если документ с таким document_no уже существует, он будет обновлен
pub async fn store_document_with_raw(
    mut document: OzonFbsPosting,
    raw_json: &str,
) -> Result<Uuid> {
    // Сохраняем сырой JSON
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "OZON",
        "OZON_FBS_Posting",
        &document.header.document_no,
        raw_json,
        document.source_meta.fetched_at,
    )
    .await?;

    // Обновляем ссылку в метаданных
    document.source_meta.raw_payload_ref = raw_ref;

    // Валидируем
    document
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    document.before_write();

    tracing::info!(
        "Saving OZON FBS document: {} (id: {}, is_deleted: {}, lines: {})",
        document.header.document_no,
        document.base.id.as_string(),
        document.base.metadata.is_deleted,
        document.lines.len()
    );

    // Сохраняем документ (upsert)
    let id = repository::upsert_document(&document).await?;
    
    tracing::info!("Successfully saved OZON FBS document with id: {}", id);
    
    // Проецируем в Sales Register
    if let Err(e) = crate::projections::p900_mp_sales_register::service::project_ozon_fbs(&document).await {
        tracing::error!("Failed to project OZON FBS document to Sales Register: {}", e);
        // Не останавливаем выполнение, т.к. документ уже сохранен
    }
    
    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<OzonFbsPosting>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<OzonFbsPosting>> {
    repository::get_by_document_no(document_no).await
}

pub async fn list_all() -> Result<Vec<OzonFbsPosting>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

