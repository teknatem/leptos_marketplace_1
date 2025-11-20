use super::repository;
use anyhow::Result;
use uuid::Uuid;

/// Провести документ (установить is_posted = true)
/// Для Orders пока нет проекций, только устанавливаем флаг
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Автозаполнение ссылок на marketplace_product и nomenclature
    super::service::auto_fill_references(&mut document).await?;

    // Установить флаг is_posted
    document.is_posted = true;
    document.base.metadata.is_posted = document.is_posted;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // TODO: Если в будущем нужны проекции для Orders, добавить их здесь
    tracing::info!("Posted WB Orders document: {}", id);

    Ok(())
}

/// Отменить проведение документа (установить is_posted = false)
pub async fn unpost_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Снять флаг is_posted
    document.is_posted = false;
    document.base.metadata.is_posted = document.is_posted;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // TODO: Если в будущем нужны проекции для Orders, удалить их здесь
    tracing::info!("Unposted WB Orders document: {}", id);

    Ok(())
}

