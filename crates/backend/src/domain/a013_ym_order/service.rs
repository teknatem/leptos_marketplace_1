use super::repository;
use anyhow::Result;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductDto;
use contracts::domain::a013_ym_order::aggregate::{YmOrder, YmOrderLine};
use contracts::domain::common::AggregateId;
use uuid::Uuid;

/// Автозаполнение marketplace_product_ref и nomenclature_ref для одной строки
/// Возвращает обновлённую строку
async fn fill_line_references(
    line: &YmOrderLine,
    connection_id: &str,
    marketplace_id: &str,
) -> Result<YmOrderLine> {
    let mut updated_line = line.clone();

    // Если уже заполнено - пропускаем
    if updated_line.marketplace_product_ref.is_some() && updated_line.nomenclature_ref.is_some() {
        return Ok(updated_line);
    }

    // Ищем marketplace_product по connection и SKU (используем offer_id или shop_sku)
    let sku = if !line.offer_id.is_empty() {
        &line.offer_id
    } else {
        &line.shop_sku
    };

    let marketplace_product =
        crate::domain::a007_marketplace_product::service::get_by_connection_and_sku(
            connection_id,
            sku,
        )
        .await?;

    let mp_id = if let Some(existing) = marketplace_product {
        existing.base.id.as_string()
    } else {
        // Создаем новый a007_marketplace_product
        let dto = MarketplaceProductDto {
            id: None,
            code: Some(format!("YM-AUTO-{}", Uuid::new_v4())),
            description: if line.name.trim().is_empty() {
                format!("Артикул: {}", sku)
            } else {
                line.name.clone()
            },
            marketplace_ref: marketplace_id.to_string(),
            connection_mp_ref: connection_id.to_string(),
            marketplace_sku: sku.to_string(),
            barcode: None,
            article: sku.to_string(),
            brand: None,
            category_id: None,
            category_name: None,
            last_update: Some(chrono::Utc::now()),
            nomenclature_ref: None,
            comment: Some(format!(
                "Автоматически создано при импорте YM Order [{}]",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            )),
        };

        let created_id = crate::domain::a007_marketplace_product::service::create(dto).await?;
        tracing::info!("Created new marketplace_product with id: {}", created_id);
        created_id.to_string()
    };

    updated_line.marketplace_product_ref = Some(mp_id.clone());

    // Получаем nomenclature_ref из marketplace_product
    if updated_line.nomenclature_ref.is_none() {
        if let Ok(mp_uuid) = Uuid::parse_str(&mp_id) {
            if let Ok(Some(mp)) =
                crate::domain::a007_marketplace_product::service::get_by_id(mp_uuid).await
            {
                if let Some(nom_ref) = mp.nomenclature_ref {
                    updated_line.nomenclature_ref = Some(nom_ref);
                }
            }
        }
    }

    // Устанавливаем price_plan = 0 если не задано
    if updated_line.price_plan.is_none() {
        updated_line.price_plan = Some(0.0);
    }

    Ok(updated_line)
}

/// Автозаполнение marketplace_product_ref и nomenclature_ref для всех строк документа
/// Обновляет поле is_error на основе наличия nomenclature_ref
pub async fn auto_fill_references(document: &mut YmOrder) -> Result<()> {
    let connection_id = &document.header.connection_id;
    let marketplace_id = &document.header.marketplace_id;

    let mut updated_lines = Vec::with_capacity(document.lines.len());

    for line in &document.lines {
        match fill_line_references(line, connection_id, marketplace_id).await {
            Ok(updated_line) => {
                updated_lines.push(updated_line);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fill references for line {}: {}",
                    line.line_id,
                    e
                );
                // Добавляем оригинальную строку без изменений
                updated_lines.push(line.clone());
            }
        }
    }

    document.lines = updated_lines;

    // Обновляем is_error на основе строк
    document.update_is_error();
    // Пересчитываем итоги
    document.recalculate_totals();

    Ok(())
}

pub async fn store_document_with_raw(mut document: YmOrder, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "YM",
        "YM_Order",
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

    tracing::info!("Successfully saved YM Order document with id: {}", id);

    // Проводим документ если is_posted = true
    if document.is_posted {
        if let Err(e) = super::posting::post_document(id).await {
            tracing::error!("Failed to post YM Order document: {}", e);
            // Не останавливаем выполнение, т.к. документ уже сохранен
        }
    } else {
        // Если is_posted = false, удаляем проекции (если были)
        if let Err(e) = crate::projections::p900_mp_sales_register::repository::delete_by_registrator(&id.to_string()).await {
            tracing::error!("Failed to delete projections for YM Order document: {}", e);
        }
    }

    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmOrder>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<YmOrder>> {
    repository::get_by_document_no(document_no).await
}

pub async fn list_all() -> Result<Vec<YmOrder>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

