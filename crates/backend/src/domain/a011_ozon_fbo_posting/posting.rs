use super::repository;
use anyhow::Result;
use uuid::Uuid;

/// Провести документ (установить is_posted = true и создать проекции)
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

    // Удалить старые проекции (если были)
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string())
        .await?;

    // Создать новые проекции
    crate::projections::p900_mp_sales_register::service::project_ozon_fbo(&document, id).await?;
    crate::projections::p904_sales_data::service::project_ozon_fbo(&document, id).await?;

    tracing::info!("Posted document a011: {} - projections created (P900 + P904)", id);
    Ok(())
}

/// Отменить проведение документа (установить is_posted = false и удалить проекции)
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

    // Удалить проекции
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string())
        .await?;

    tracing::info!("Unposted document a011: {}", id);
    Ok(())
}
