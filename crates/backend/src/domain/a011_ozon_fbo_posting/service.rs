use super::repository;
use anyhow::Result;
use contracts::domain::a011_ozon_fbo_posting::aggregate::OzonFboPosting;
use uuid::Uuid;

pub async fn store_document_with_raw(
    mut document: OzonFboPosting,
    raw_json: &str,
) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "OZON",
        "OZON_FBO_Posting",
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

    tracing::info!("Successfully saved OZON FBO document with id: {}", id);

    // Проводим документ если is_posted = true
    if document.is_posted {
        if let Err(e) = super::posting::post_document(id).await {
            tracing::error!("Failed to post OZON FBO document: {}", e);
            // Не останавливаем выполнение, т.к. документ уже сохранен
        }
    } else {
        // Если is_posted = false, удаляем проекции (если были)
        if let Err(e) = crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string()).await {
            tracing::error!("Failed to delete projections for OZON FBO document: {}", e);
        }
    }

    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<OzonFboPosting>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<OzonFboPosting>> {
    repository::get_by_document_no(document_no).await
}

pub async fn list_all() -> Result<Vec<OzonFboPosting>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

