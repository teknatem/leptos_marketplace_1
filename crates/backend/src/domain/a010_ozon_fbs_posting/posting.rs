use super::repository;
use anyhow::Result;
use uuid::Uuid;

/// Провести документ (установить is_posted = true и создать проекции если статус DELIVERED)
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Установить флаг is_posted (для любого статуса)
    document.is_posted = true;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // Удалить старые проекции (если были)
    crate::projections::p900_mp_sales_register::repository::delete_by_registrator(
        &id.to_string(),
    )
    .await?;

    // Создать новые проекции только для DELIVERED
    if document.state.status_norm == "DELIVERED" {
        crate::projections::p900_mp_sales_register::service::project_ozon_fbs(&document, id).await?;
        tracing::info!("Posted document a010: {} with status DELIVERED - projections created", id);
    } else {
        tracing::info!("Posted document a010: {} with status {} - no projections created", id, document.state.status_norm);
    }

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
    crate::projections::p900_mp_sales_register::repository::delete_by_registrator(
        &id.to_string(),
    )
    .await?;

    tracing::info!("Unposted document a010: {}", id);
    Ok(())
}
