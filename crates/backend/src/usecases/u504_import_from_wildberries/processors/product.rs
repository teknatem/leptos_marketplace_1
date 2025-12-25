use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;
use contracts::domain::common::AggregateId;
use crate::domain::a007_marketplace_product;
use super::super::wildberries_api_client::WildberriesCard;

/// Обработать один товар (upsert)
pub async fn process_product(
    connection: &ConnectionMP,
    card: &WildberriesCard,
) -> Result<bool> {
    // Используем nm_id как marketplace_sku
    let marketplace_sku = card.nm_id.to_string();
    let existing = a007_marketplace_product::repository::get_by_connection_and_sku(
        &connection.base.id.as_string(),
        &marketplace_sku,
    )
    .await?;

    // Берем первый barcode из списка sizes
    let barcode = card.sizes.first().and_then(|s| s.barcode.clone());

    // Получаем название товара для description
    let product_title = card
        .title
        .clone()
        .unwrap_or_else(|| "Без названия".to_string());

    if let Some(mut existing_product) = existing {
        // Обновляем существующий товар
        tracing::debug!("Updating existing product: {}", marketplace_sku);

        existing_product.base.code = card.vendor_code.clone();
        existing_product.base.description = product_title.clone();
        existing_product.marketplace_sku = marketplace_sku;
        existing_product.barcode = barcode.clone();
        existing_product.article = card.vendor_code.clone();
        existing_product.brand = card.brand.clone();
        existing_product.category_id = Some(card.subject_id.to_string());
        existing_product.category_name = None; // WB API не возвращает название категории
        existing_product.last_update = Some(chrono::Utc::now());
        existing_product.before_write();

        a007_marketplace_product::repository::update(&existing_product).await?;
        Ok(false)
    } else {
        // Создаем новый товар
        tracing::debug!("Inserting new product: {}", marketplace_sku);

        let mut new_product = MarketplaceProduct::new_for_insert(
            card.vendor_code.clone(),
            product_title.clone(),
            connection.marketplace_id.clone(),
            connection.base.id.as_string(),
            marketplace_sku,
            barcode,
            card.vendor_code.clone(),
            card.brand.clone(),
            Some(card.subject_id.to_string()),
            None, // category_name
            Some(chrono::Utc::now()),
            None, // nomenclature_ref
            None, // comment
        );

        // Автоматический поиск номенклатуры по артикулу
        let _ =
            a007_marketplace_product::service::search_and_set_nomenclature(&mut new_product)
                .await;

        a007_marketplace_product::repository::insert(&new_product).await?;
        Ok(true)
    }
}

