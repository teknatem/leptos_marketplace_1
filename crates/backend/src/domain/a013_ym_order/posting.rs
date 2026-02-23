use super::repository;
use super::service::auto_fill_references;
use anyhow::Result;
use uuid::Uuid;

/// Провести документ (установить is_posted = true и создать проекции)
/// При проведении автоматически заполняются:
/// - marketplace_product_ref (поиск или создание в a007_marketplace_product)
/// - nomenclature_ref (из соответствия в a007_marketplace_product)
/// - is_error (ненулевой если есть строки без nomenclature_ref)
/// - Недостающие поля (creation_date, delivery_date и т.д.) из raw JSON для старых документов
pub async fn post_document(id: Uuid) -> Result<()> {
    // Загрузить документ (с полными строками из items table)
    let mut document = repository::get_by_id_with_items(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    // Заполнить отсутствующие поля из raw JSON (для старых документов)
    let fields_refilled = super::service::refill_from_raw_json(&mut document).await?;
    if fields_refilled {
        tracing::info!(
            "Refilled missing fields from raw JSON for document {}",
            document.header.document_no
        );
    }

    // Автозаполнение ссылок для всех строк
    auto_fill_references(&mut document).await?;

    // Заполнение dealer_price_ut для каждой строки из p906_nomenclature_prices
    super::service::fill_dealer_price_for_lines(&mut document).await?;

    // Расчёт итоговой дилерской суммы и маржи документа
    super::service::calculate_totals_and_margin(&mut document).await?;

    // Установить флаг is_posted
    document.is_posted = true;
    document.before_write();

    // Сохранить документ (включая обновлённые строки в items table)
    repository::upsert_document(&document).await?;

    // Удалить старые проекции (если были)
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string())
        .await?;

    // Создать новые проекции
    crate::projections::p900_mp_sales_register::service::project_ym_order(&document, id).await?;
    crate::projections::p904_sales_data::service::project_ym_order(&document, id).await?;

    tracing::info!(
        "Posted document a013: {}, is_error: {}",
        id,
        document.is_error
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
    document.is_posted = false;
    document.before_write();

    // Сохранить документ
    repository::upsert_document(&document).await?;

    // Удалить проекции
    crate::projections::p900_mp_sales_register::service::delete_by_registrator(&id.to_string())
        .await?;
    crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string())
        .await?;

    tracing::info!("Unposted document a013: {}", id);
    Ok(())
}
