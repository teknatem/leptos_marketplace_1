use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;
use contracts::domain::common::AggregateId;
use crate::domain::a007_marketplace_product;
use super::super::yandex_api_client::{YandexOffer, YandexMapping};

/// Обработать один товар из YandexOffer (offer-mappings endpoint)
/// Возвращает (is_new, barcodes_count)
pub async fn process_product_from_offer(
    connection: &ConnectionMP,
    offer: &YandexOffer,
    _mapping: &Option<YandexMapping>,
) -> Result<(bool, usize)> {
    // Используем offer_id as marketplace_sku
    let marketplace_sku = offer.offer_id.clone();
    let existing = a007_marketplace_product::repository::get_by_connection_and_sku(
        &connection.base.id.as_string(),
        &marketplace_sku,
    )
    .await?;

    // Берем первый barcode из списка
    let barcode = offer.barcodes.first().cloned();

    // Получаем category_id и category_name - YandexMapping не содержит категории
    let (category_id, category_name) = (None, offer.category.clone());

    // Получаем название товара из offer.name для description
    let product_title = offer
        .name
        .clone()
        .unwrap_or_else(|| "Без названия".to_string());

    if let Some(mut existing_product) = existing {
        // Обновляем существующий товар
        tracing::debug!("Updating existing product: {}", marketplace_sku);

        existing_product.base.code = offer.offer_id.clone();
        existing_product.base.description = product_title.clone();
        existing_product.marketplace_sku = marketplace_sku;
        existing_product.barcode = barcode.clone();
        existing_product.article = offer.offer_id.clone();
        existing_product.brand = offer.vendor.clone();
        existing_product.category_id = category_id;
        existing_product.category_name = category_name;
        existing_product.last_update = Some(chrono::Utc::now());
        existing_product.before_write();

        a007_marketplace_product::repository::update(&existing_product).await?;

        // Импорт всех штрихкодов в проекцию p901
        let barcodes_count = import_barcodes_to_p901(
            &offer.barcodes,
            &offer.offer_id,
            &existing_product.nomenclature_ref,
        )
        .await?;

        Ok((false, barcodes_count))
    } else {
        // Создаем новый товар
        tracing::debug!("Inserting new product: {}", marketplace_sku);

        let mut new_product = MarketplaceProduct::new_for_insert(
            offer.offer_id.clone(),
            product_title.clone(),
            connection.marketplace_id.clone(),
            connection.base.id.as_string(),
            marketplace_sku,
            barcode,
            offer.offer_id.clone(),
            offer.vendor.clone(),
            category_id,
            category_name,
            Some(chrono::Utc::now()),
            None, // nomenclature_ref
            None, // comment
        );

        // Автоматический поиск номенклатуры по артикулу
        let _ =
            a007_marketplace_product::service::search_and_set_nomenclature(&mut new_product)
                .await;

        a007_marketplace_product::repository::insert(&new_product).await?;

        // Импорт всех штрихкодов в проекцию p901
        let barcodes_count = import_barcodes_to_p901(
            &offer.barcodes,
            &offer.offer_id,
            &new_product.nomenclature_ref,
        )
        .await?;

        Ok((true, barcodes_count))
    }
}

/// Импортировать все штрихкоды из Yandex в проекцию p901_nomenclature_barcodes
pub async fn import_barcodes_to_p901(
    barcodes: &[String],
    article: &str,
    product_nomenclature_id: &Option<String>,
) -> Result<usize> {
    use crate::projections::p901_nomenclature_barcodes::{repository, service};

    if barcodes.is_empty() {
        return Ok(0);
    }

    let mut imported_count = 0;

    for barcode in barcodes {
        if barcode.trim().is_empty() {
            continue;
        }

        let nomenclature_ref = if let Some(ref nom_id) = product_nomenclature_id {
            Some(nom_id.clone())
        } else {
            match service::find_nomenclature_ref_by_barcode_from_1c(barcode).await {
                Ok(found_ref) => found_ref,
                Err(_) => None,
            }
        };

        let entry = match service::create_entry(
            barcode.clone(),
            "YM".to_string(),
            nomenclature_ref.clone(),
            Some(article.to_string()),
        ) {
            Ok(e) => e,
            Err(_) => continue,
        };

        match repository::upsert_entry(&entry).await {
            Ok(_) => { imported_count += 1; }
            Err(_) => {}
        }
    }

    Ok(imported_count)
}

