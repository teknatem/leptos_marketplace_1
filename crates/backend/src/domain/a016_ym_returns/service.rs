use super::repository;
use anyhow::Result;
use contracts::domain::a016_ym_returns::aggregate::YmReturn;
use uuid::Uuid;

pub async fn store_document_with_raw(mut document: YmReturn, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "YM",
        "YM_Return",
        &document.header.return_id.to_string(),
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

    tracing::info!("Successfully saved YM Return document with id: {}", id);

    // Проводим документ если is_posted = true
    if document.is_posted {
        if let Err(e) = super::posting::post_document(id).await {
            tracing::error!("Failed to post YM Return document: {}", e);
            // Не останавливаем выполнение, т.к. документ уже сохранен
        }
    }

    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmReturn>> {
    repository::get_by_id(id).await
}

pub async fn get_by_return_id(return_id: i64) -> Result<Option<YmReturn>> {
    repository::get_by_return_id(return_id).await
}

pub async fn list_all() -> Result<Vec<YmReturn>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

