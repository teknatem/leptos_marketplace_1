use super::repository;
use anyhow::Result;
use uuid::Uuid;

/// Провести документ (установить is_posted = true и создать проекции P904)
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Найти документ постинга по posting_number
    let posting_number = &document.posting.posting_number;
    
    // Сначала пробуем найти A010 (FBS)
    let posting_fbs = crate::domain::a010_ozon_fbs_posting::service::get_by_document_no(posting_number).await?;
    
    // Если не нашли, пробуем A011 (FBO)
    let posting_fbo = if posting_fbs.is_none() {
        crate::domain::a011_ozon_fbo_posting::service::get_by_document_no(posting_number).await?
    } else {
        None
    };
    
    // Установить ссылку на постинг, если найден
    if let Some(fbs_doc) = posting_fbs {
        document.posting_ref = Some(fbs_doc.to_string_id());
        document.posting_ref_type = Some("A010".to_string());
        tracing::info!("Found A010 FBS posting for transaction: {}", posting_number);
    } else if let Some(fbo_doc) = posting_fbo {
        document.posting_ref = Some(fbo_doc.to_string_id());
        document.posting_ref_type = Some("A011".to_string());
        tracing::info!("Found A011 FBO posting for transaction: {}", posting_number);
    } else {
        tracing::warn!("Posting document not found for posting_number: {}", posting_number);
        // Оставляем posting_ref и posting_ref_type как None
    }

    // Обогатить items данными из постинга
    super::service_enrichment::enrich_items_from_posting(&mut document).await?;

    // Установить флаг is_posted
    document.is_posted = true;
    document.before_write();

    // Сохранить документ
    repository::upsert_by_operation_id(&document).await?;

    // Удалить старые проекции (если были)
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string())
        .await?;

    // Создать новые проекции в P904
    crate::projections::p904_sales_data::service::project_ozon_transactions(&document, id).await?;

    tracing::info!("Posted document a014: {} - P904 projections created", id);

    Ok(())
}

/// Отменить проведение документа (установить is_posted = false и удалить проекции)
pub async fn unpost_document(id: Uuid) -> Result<()> {
    // Загрузить документ
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Снять флаг is_posted и очистить ссылки на постинг
    document.is_posted = false;
    document.posting_ref = None;
    document.posting_ref_type = None;
    document.before_write();

    // Сохранить документ
    repository::upsert_by_operation_id(&document).await?;

    // Удалить проекции
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string())
        .await?;

    tracing::info!("Unposted document a014: {}", id);
    Ok(())
}

