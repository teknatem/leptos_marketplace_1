use crate::domain::a007_marketplace_product;
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;
use contracts::domain::common::AggregateId;
use super::super::ozon_api_client::OzonProductInfo;

/// Обработать один товар (upsert)
pub async fn process_product(
    connection: &ConnectionMP,
    product: &OzonProductInfo,
) -> Result<bool> {
    // Проверяем, существует ли товар по marketplace_sku (product_id)
    let marketplace_sku = product.id.to_string();
    let existing = a007_marketplace_product::repository::get_by_connection_and_sku(
        &connection.base.id.as_string(),
        &marketplace_sku,
    )
    .await?;

    // Берем первый barcode из списка
    let barcode = product.barcodes.first().cloned();

    // Получаем category_id
    let category_id = product.description_category_id.map(|id| id.to_string());

    if let Some(mut existing_product) = existing {
        // Обновляем существующий товар
        tracing::debug!("Updating existing product: {}", marketplace_sku);

        existing_product.base.code = product.offer_id.clone();
        existing_product.base.description = product.name.clone();
        existing_product.marketplace_sku = marketplace_sku;
        existing_product.barcode = barcode.clone();
        existing_product.article = product.offer_id.clone();
        existing_product.category_id = category_id.clone();
        existing_product.last_update = Some(chrono::Utc::now());
        existing_product.before_write();

        a007_marketplace_product::repository::update(&existing_product).await?;
        Ok(false)
    } else {
        // Создаем новый товар
        tracing::debug!("Inserting new product: {}", marketplace_sku);

        let mut new_product = MarketplaceProduct::new_for_insert(
            product.offer_id.clone(),
            product.name.clone(),
            connection.marketplace_id.clone(),
            connection.base.id.as_string(),
            marketplace_sku,
            barcode,
            product.offer_id.clone(),
            None, // brand
            category_id,
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

