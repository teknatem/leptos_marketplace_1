use super::repository;
use anyhow::Result;
use uuid::Uuid;

/// Провести документ (установить is_posted = true и создать проекцию с отрицательными значениями)
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Установить флаг is_posted
    document.base.metadata.is_posted = true;
    document.before_write();

    // Сохранить документ
    repository::update(&document).await?;

    // Удалить старые проекции (если были)
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;

    // Создать новую проекцию (возврат = отрицательные значения в Sales Register)
    crate::projections::p900_mp_sales_register::service::project_ozon_returns(&document, id).await?;

    tracing::info!(
        "Posted document a009 (OZON Return): {} - projection created with negative qty: -{}",
        id,
        document.quantity
    );

    Ok(())
}

/// Отменить проведение документа (установить is_posted = false и удалить проекции)
pub async fn unpost_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Снять флаг is_posted
    document.base.metadata.is_posted = false;
    document.before_write();

    // Сохранить документ
    repository::update(&document).await?;

    // Удалить проекции
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;

    tracing::info!("Unposted document a009 (OZON Return): {}", id);
    Ok(())
}
