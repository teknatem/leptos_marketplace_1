use super::repository;
use anyhow::Result;
use uuid::Uuid;

/// Провести документ (установить is_posted = true)
/// Возвраты YM пока не формируют проекции в p900
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Установить флаг is_posted
    document.is_posted = true;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // TODO: Если нужно, добавить проекции для возвратов
    // Например, в отдельную таблицу p9XX_ym_returns_register

    tracing::info!("Posted document a016 (YM Return): {}", id);
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
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // TODO: Если были проекции - удалить их

    tracing::info!("Unposted document a016 (YM Return): {}", id);
    Ok(())
}

