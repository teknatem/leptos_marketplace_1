use super::repository;
use anyhow::Result;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::common::AggregateId;
use uuid::Uuid;

/// Автозаполнение marketplace_product_ref и nomenclature_ref
pub async fn auto_fill_references(document: &mut WbSales) -> Result<()> {
    // Автозаполнение marketplace_product_ref если пустой
    if document.marketplace_product_ref.is_none() {
        // Ищем по connection_mp_ref и supplier_article (используем как marketplace_sku)
        let marketplace_product =
            crate::domain::a007_marketplace_product::service::get_by_connection_and_sku(
                &document.header.connection_id,
                &document.line.supplier_article,
            )
            .await?;

        let mp_id = if let Some(existing) = marketplace_product {
            existing.base.id.as_string()
        } else {
            // Создаем новый a007_marketplace_product
            let dto =
                contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductDto {
                    id: None,
                    code: Some(format!("WB-AUTO-{}", uuid::Uuid::new_v4())),
                    description: if document.line.name.trim().is_empty() {
                        format!("Артикул: {}", document.line.supplier_article)
                    } else {
                        document.line.name.clone()
                    },
                    marketplace_ref: document.header.marketplace_id.clone(),
                    connection_mp_ref: document.header.connection_id.clone(),
                    marketplace_sku: document.line.supplier_article.clone(),
                    barcode: Some(document.line.barcode.clone()),
                    article: document.line.supplier_article.clone(),
                    brand: None,
                    category_id: None,
                    category_name: None,
                    last_update: Some(chrono::Utc::now()),
                    nomenclature_ref: None,
                    comment: Some(format!(
                        "Автоматически создано при импорте WB Sales [{}]",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                    )),
                };

            let created_id = crate::domain::a007_marketplace_product::service::create(dto).await?;
            tracing::info!("Created new marketplace_product with id: {}", created_id);
            created_id.to_string()
        };

        document.marketplace_product_ref = Some(mp_id);
    }

    // Автозаполнение nomenclature_ref из marketplace_product
    if document.nomenclature_ref.is_none() {
        if let Some(ref mp_ref) = document.marketplace_product_ref {
            if let Ok(mp_uuid) = uuid::Uuid::parse_str(mp_ref) {
                if let Ok(Some(mp)) =
                    crate::domain::a007_marketplace_product::service::get_by_id(mp_uuid).await
                {
                    if let Some(nom_ref) = mp.nomenclature_ref {
                        document.nomenclature_ref = Some(nom_ref);
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn store_document_with_raw(mut document: WbSales, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "WB",
        "WB_Sales",
        &document.header.document_no,
        raw_json,
        document.source_meta.fetched_at,
    )
    .await?;

    document.source_meta.raw_payload_ref = raw_ref;

    // Автозаполнение ссылок
    auto_fill_references(&mut document).await?;

    document
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    document.before_write();

    let id = repository::upsert_document(&document).await?;

    // Проводим документ если is_posted = true
    if document.is_posted {
        if let Err(e) = super::posting::post_document(id).await {
            tracing::error!("Failed to post WB Sales document: {}", e);
            // Не останавливаем выполнение, т.к. документ уже сохранен
        }
    } else {
        // Если is_posted = false, удаляем проекции (если были)
        if let Err(e) =
            crate::projections::p900_mp_sales_register::repository::delete_by_registrator(
                &id.to_string(),
            )
            .await
        {
            tracing::error!("Failed to delete projections for WB Sales document: {}", e);
        }
    }

    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSales>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<WbSales>> {
    repository::get_by_document_no(document_no).await
}

/// Get by sale_id (saleID from WB API) - used for deduplication
pub async fn get_by_sale_id(sale_id: &str) -> Result<Option<WbSales>> {
    repository::get_by_sale_id(sale_id).await
}

pub async fn list_all() -> Result<Vec<WbSales>> {
    repository::list_all().await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}
