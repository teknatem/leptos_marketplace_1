use super::repository;
use anyhow::Result;
use chrono::NaiveDate;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::domain::common::AggregateId;
use uuid::Uuid;

/// Автозаполнение marketplace_product_ref и nomenclature_ref
pub async fn auto_fill_references(document: &mut WbOrders) -> Result<()> {
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
                    description: if let Some(ref brand) = document.line.brand {
                        if !brand.trim().is_empty() {
                            brand.clone()
                        } else {
                            format!("Артикул: {}", document.line.supplier_article)
                        }
                    } else {
                        format!("Артикул: {}", document.line.supplier_article)
                    },
                    marketplace_ref: document.header.marketplace_id.clone(),
                    connection_mp_ref: document.header.connection_id.clone(),
                    marketplace_sku: document.line.supplier_article.clone(),
                    barcode: Some(document.line.barcode.clone()),
                    article: document.line.supplier_article.clone(),
                    brand: document.line.brand.clone(),
                    category_id: None,
                    category_name: document.line.category.clone(),
                    last_update: Some(chrono::Utc::now()),
                    nomenclature_ref: None,
                    comment: Some(format!(
                        "Автоматически создано при импорте WB Orders [{}]",
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

    refill_base_nomenclature_ref(document).await?;

    Ok(())
}

/// Принудительно пересчитывает base_nomenclature_ref по алгоритму:
/// 1) если у nomenclature_ref заполнен base_nomenclature_ref -> используем его
/// 2) иначе используем сам nomenclature_ref
pub async fn refill_base_nomenclature_ref(document: &mut WbOrders) -> Result<()> {
    const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";
    document.base_nomenclature_ref = None;

    if let Some(ref nom_ref) = document.nomenclature_ref {
        document.base_nomenclature_ref = Some(nom_ref.clone());
        if let Ok(nom_uuid) = uuid::Uuid::parse_str(nom_ref) {
            if let Ok(Some(nomenclature)) =
                crate::domain::a004_nomenclature::service::get_by_id(nom_uuid).await
            {
                if let Some(base_ref) = nomenclature.base_nomenclature_ref {
                    if !base_ref.is_empty() && base_ref != ZERO_UUID {
                        document.base_nomenclature_ref = Some(base_ref);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Автозаполнение dealer_price_ut из p906_nomenclature_prices.
/// Логика аналогична a012_wb_sales:
/// 1. Цена на дату по nomenclature_ref
/// 2. Цена на дату по base_nomenclature_ref
/// 3. Первая ненулевая цена по nomenclature_ref
/// 4. Первая ненулевая цена по base_nomenclature_ref
pub async fn fill_dealer_price(document: &mut WbOrders) -> Result<()> {
    const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

    let Some(ref nom_ref) = document.nomenclature_ref else {
        document.line.dealer_price_ut = None;
        return Ok(());
    };

    let order_date = document.state.order_dt.format("%Y-%m-%d").to_string();
    let mut price_source = String::new();

    let mut price = crate::projections::p906_nomenclature_prices::repository::get_price_for_date(
        nom_ref,
        &order_date,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("Failed to get dealer price for {}: {}", nom_ref, e);
        None
    });

    if price.is_some() && price.unwrap_or(0.0) > 0.0 {
        price_source = format!("nomenclature {} on date {}", nom_ref, order_date);
    } else {
        price = None;
    }

    if price.is_none() {
        if let Ok(nom_uuid) = Uuid::parse_str(nom_ref) {
            if let Ok(Some(nomenclature)) =
                crate::domain::a004_nomenclature::service::get_by_id(nom_uuid).await
            {
                if let Some(ref base_ref) = nomenclature.base_nomenclature_ref {
                    if !base_ref.is_empty() && base_ref != ZERO_UUID {
                        price = crate::projections::p906_nomenclature_prices::repository::get_price_for_date(
                            base_ref,
                            &order_date,
                        )
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!(
                                "Failed to get dealer price for base_nomenclature {}: {}",
                                base_ref,
                                e
                            );
                            None
                        });

                        if price.is_some() && price.unwrap_or(0.0) > 0.0 {
                            price_source =
                                format!("base_nomenclature {} on date {}", base_ref, order_date);
                        } else {
                            price = None;
                        }
                    }
                }
            }
        }
    }

    if price.is_none() {
        price = crate::projections::p906_nomenclature_prices::repository::get_first_nonzero_price(
            nom_ref,
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to get first nonzero price for {}: {}", nom_ref, e);
            None
        });

        if price.is_some() {
            price_source = format!("nomenclature {} (first nonzero price)", nom_ref);
        }
    }

    if price.is_none() {
        if let Ok(nom_uuid) = Uuid::parse_str(nom_ref) {
            if let Ok(Some(nomenclature)) =
                crate::domain::a004_nomenclature::service::get_by_id(nom_uuid).await
            {
                if let Some(ref base_ref) = nomenclature.base_nomenclature_ref {
                    if !base_ref.is_empty() && base_ref != ZERO_UUID {
                        price = crate::projections::p906_nomenclature_prices::repository::get_first_nonzero_price(
                            base_ref,
                        )
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!(
                                "Failed to get first nonzero price for base_nomenclature {}: {}",
                                base_ref,
                                e
                            );
                            None
                        });

                        if price.is_some() {
                            price_source =
                                format!("base_nomenclature {} (first nonzero price)", base_ref);
                        }
                    }
                }
            }
        }
    }

    if price.is_some() {
        tracing::info!(
            "Filled dealer_price_ut = {:?} for WB Orders document {} (from {})",
            price,
            document.base.id.as_string(),
            price_source
        );
    } else {
        tracing::warn!(
            "Could not find dealer_price_ut for WB Orders document {} (nomenclature: {})",
            document.base.id.as_string(),
            nom_ref
        );
    }

    document.line.dealer_price_ut = price;
    Ok(())
}

/// Расчёт margin_pro в процентах, если dealer_price_ut > 0:
/// 1) Основная формула:
///    (price_with_disc * (100 - planned_commission_percent) / 100 - dealer_price_ut)
///    / dealer_price_ut * 100
/// 2) Fallback при отсутствии planned_commission_percent:
///    (finished_price - dealer_price_ut) / dealer_price_ut * 100
pub async fn calculate_margin_pro(document: &mut WbOrders) -> Result<()> {
    let dealer_price = document.line.dealer_price_ut.unwrap_or(0.0);
    if dealer_price <= 0.0 {
        document.line.margin_pro = None;
        return Ok(());
    }

    // Базовый fallback: старая формула.
    let finished_price = document.line.finished_price.unwrap_or(0.0);
    let mut margin = (finished_price - dealer_price) / dealer_price * 100.0;

    match Uuid::parse_str(&document.header.connection_id) {
        Ok(connection_id) => {
            match crate::domain::a006_connection_mp::service::get_by_id(connection_id).await {
                Ok(Some(connection)) => {
                    if let Some(planned_percent) = connection.planned_commission_percent {
                        let price_with_disc = document.line.price_with_disc.unwrap_or(0.0);
                        margin = (price_with_disc * (100.0 - planned_percent) / 100.0
                            - dealer_price)
                            / dealer_price
                            * 100.0;
                    }
                }
                Ok(None) => {
                    tracing::warn!(
                        "Connection MP not found for WB Orders document {}, id: {}. Using legacy margin formula.",
                        document.base.id.as_string(),
                        document.header.connection_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load Connection MP for WB Orders document {}: {}. Using legacy margin formula.",
                        document.base.id.as_string(),
                        e
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "Invalid connection_id {} for WB Orders document {}: {}. Using legacy margin formula.",
                document.header.connection_id,
                document.base.id.as_string(),
                e
            );
        }
    }

    document.line.margin_pro = Some(margin);
    Ok(())
}

pub async fn store_document_with_raw(mut document: WbOrders, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "WB",
        "WB_Orders",
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
            tracing::error!("Failed to post WB Orders document: {}", e);
            // Не останавливаем выполнение, т.к. документ уже сохранен
        }
    }

    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbOrders>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<WbOrders>> {
    repository::get_by_document_no(document_no).await
}

pub async fn list_all() -> Result<Vec<WbOrders>> {
    repository::list_all().await
}

pub async fn list_by_date_range(
    date_from: Option<NaiveDate>,
    date_to: Option<NaiveDate>,
) -> Result<Vec<WbOrders>> {
    repository::list_by_date_range(date_from, date_to).await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}
