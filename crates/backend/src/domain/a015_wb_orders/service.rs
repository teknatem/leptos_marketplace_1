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
        if let Some(marketplace_sku) =
            crate::domain::a007_marketplace_product::service::wb_marketplace_sku(
                document.line.nm_id,
            )
        {
            let mp_id = crate::domain::a007_marketplace_product::service::find_or_create_for_sale(
                crate::domain::a007_marketplace_product::service::FindOrCreateParams {
                    marketplace_ref: document.header.marketplace_id.clone(),
                    connection_mp_ref: document.header.connection_id.clone(),
                    marketplace_sku,
                    article: Some(document.line.supplier_article.clone()),
                    barcode: Some(document.line.barcode.clone()),
                    title: if let Some(ref brand) = document.line.brand {
                        if !brand.trim().is_empty() {
                            brand.clone()
                        } else {
                            format!("Артикул: {}", document.line.supplier_article)
                        }
                    } else {
                        format!("Артикул: {}", document.line.supplier_article)
                    },
                },
            )
            .await?;

            document.marketplace_product_ref = Some(mp_id.to_string());
        }
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

pub async fn fill_dealer_price_resolved(document: &mut WbOrders) -> Result<()> {
    let Some(ref nom_ref) = document.nomenclature_ref else {
        document.line.dealer_price_ut = None;
        return Ok(());
    };

    let order_date = document.state.order_dt.format("%Y-%m-%d").to_string();
    let resolved =
        crate::projections::p906_nomenclature_prices::service::resolve_price_for_nomenclature(
            nom_ref,
            &order_date,
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to resolve dealer price for {}: {}", nom_ref, e);
            None
        });

    if let Some(ref resolved_price) = resolved {
        tracing::info!(
            "Filled dealer_price_ut = {:?} for WB Orders document {} (from {})",
            resolved_price.price,
            document.base.id.as_string(),
            resolved_price.describe(&order_date)
        );
    } else {
        tracing::warn!(
            "Could not find dealer_price_ut for WB Orders document {} (nomenclature: {})",
            document.base.id.as_string(),
            nom_ref
        );
    }

    document.line.dealer_price_ut = resolved.map(|resolved_price| resolved_price.price);
    Ok(())
}

/// Расчёт margin_pro в процентах, если dealer_price_ut и price_with_disc > 0:
/// (price_with_disc - dealer_price_ut) / price_with_disc * 100
pub async fn calculate_margin_pro(document: &mut WbOrders) -> Result<()> {
    let dealer_price = document.line.dealer_price_ut.unwrap_or(0.0);
    let price_with_disc = document.line.price_with_disc.unwrap_or(0.0);

    if dealer_price <= 0.0 || price_with_disc <= 0.0 {
        document.line.margin_pro = None;
        return Ok(());
    }

    let margin = (price_with_disc - dealer_price) / price_with_disc * 100.0;
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

    // Preserve income_id and numeric order ID set by Marketplace API.
    // Marketplace API populates supplyId in real-time; statistics API lags 1-3 days.
    // Also, marketplace API stores the numeric WB order ID in line_id; statistics API only has srid.
    let needs_income_check = document.source_meta.income_id.is_none();
    // Statistics API sets line_id to srid (non-numeric); marketplace set it to digits-only order ID.
    let current_line_id_is_numeric = document.line.line_id.chars().all(|c| c.is_ascii_digit());
    if needs_income_check || !current_line_id_is_numeric {
        if let Ok(Some(existing)) =
            repository::get_by_document_no(&document.header.document_no).await
        {
            if needs_income_check {
                if let Some(existing_income_id) = existing.source_meta.income_id {
                    if existing_income_id != 0 {
                        document.source_meta.income_id = Some(existing_income_id);
                    }
                }
            }
            // Preserve numeric WB order ID stored by marketplace import
            if !current_line_id_is_numeric
                && existing.line.line_id.chars().all(|c| c.is_ascii_digit())
                && !existing.line.line_id.is_empty()
            {
                document.line.line_id = existing.line.line_id;
            }
        }
    }

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

/// Find orders that belong to a supply, matched by incomeID from WB Statistics API.
/// Supply ID like "WB-GI-229481414" → income_id = 229481414.
pub async fn list_by_income_id(income_id: i64) -> Result<Vec<WbOrders>> {
    repository::list_by_income_id(income_id).await
}

pub async fn list_by_numeric_order_ids(order_ids: &[i64]) -> Result<Vec<WbOrders>> {
    repository::list_by_numeric_order_ids(order_ids).await
}

/// Update income_id for an order by its document_no (srid).
/// Called when marketplace API provides supply assignment in real-time.
pub async fn update_income_id_by_document_no(document_no: &str, income_id: i64) -> Result<bool> {
    repository::update_income_id_by_document_no(document_no, income_id).await
}

pub async fn set_income_id_by_document_no(
    document_no: &str,
    income_id: Option<i64>,
) -> Result<bool> {
    repository::set_income_id_by_document_no(document_no, income_id).await
}

/// Update numeric WB order ID (line_id) for an existing order.
/// Only updates if line_id is currently non-numeric (e.g. srid from Statistics API).
pub async fn update_line_id_by_document_no(document_no: &str, line_id: i64) -> Result<()> {
    repository::update_line_id_by_document_no(document_no, line_id).await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}
