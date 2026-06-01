use super::super::wildberries_api_client::WbMarketplaceOrderRow;
use crate::domain::a015_wb_orders;
use crate::shared::marketplaces::wildberries::datetime::{
    format_wb_local_datetime_seconds, parse_wb_datetime,
};
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a015_wb_orders::aggregate::{
    WbOrders, WbOrdersGeography, WbOrdersHeader, WbOrdersLine, WbOrdersSourceMeta, WbOrdersState,
    WbOrdersWarehouse,
};
use contracts::domain::common::AggregateId;

/// Process a single order from /api/v3/orders or /api/v3/orders/new.
///
/// Strategy:
/// - If the order doesn't exist in a015 yet → INSERT with partial marketplace data.
///   Financial fields (prices, discounts, geography) will be filled later by Statistics API.
/// - If the order already exists → only update income_id (if not already set) and preserve
///   all financial data from Statistics API.
///
/// Returns true if a new record was inserted.
pub async fn process_marketplace_order(
    connection: &ConnectionMP,
    organization_id: &str,
    order: &WbMarketplaceOrderRow,
) -> Result<bool> {
    // `rid` is the srid equivalent — used as document_no (unique key)
    let document_no = match &order.rid {
        Some(rid) if !rid.is_empty() => rid.clone(),
        _ => {
            // Fall back to "WB_MP_{id}" if rid is missing
            format!("WB_MP_{}", order.id)
        }
    };

    // Check if the order already exists
    let existing = a015_wb_orders::service::get_by_document_no(&document_no).await?;

    if let Some(existing) = existing {
        // Order exists — update income_id and store numeric WB order ID for sticker API
        if order.id > 0 {
            let _ = a015_wb_orders::service::update_line_id_by_document_no(&document_no, order.id)
                .await;
        }
        if let Some(ref supply_id) = order.supply_id {
            if let Some(income_id) = supply_id
                .rsplit('-')
                .next()
                .and_then(|s| s.parse::<i64>().ok())
                .filter(|&v| v > 0)
            {
                a015_wb_orders::service::update_income_id_by_document_no(&document_no, income_id)
                    .await?;
            }
        }
        // Only overwrite the marketplace raw payload when the new data is at least as rich as
        // the existing one.  /api/v3/orders/new returns salePrice and finalPrice; the historical
        // /api/v3/orders endpoint returns them as null.  If we already have richer data stored
        // (non-null salePrice or finalPrice) and the new payload lacks those fields, keep the
        // existing payload to prevent losing price information.
        let new_has_rich_prices = order.final_price.is_some() || order.sale_price.is_some();
        let existing_has_rich_prices = existing.source_meta.marketplace_raw_payload_ref.is_some();
        if new_has_rich_prices || !existing_has_rich_prices {
            let raw_json = serde_json::to_string(order)?;
            let _ = a015_wb_orders::service::store_marketplace_raw_payload(&document_no, &raw_json)
                .await?;
        }
        if let Some(price_rub) = order.price.filter(|&p| p > 0).map(|p| p as f64 / 100.0) {
            let _ = a015_wb_orders::service::update_line_price_if_missing(&document_no, price_rub)
                .await;
        }
        return Ok(false);
    }

    // New order — create record with whatever marketplace data is available
    let header = WbOrdersHeader {
        document_no: document_no.clone(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let supplier_article = order.article.clone().unwrap_or_default();
    let nm_id = order.nm_id.unwrap_or(0);
    let barcode = order
        .skus
        .as_ref()
        .and_then(|s| s.first())
        .cloned()
        .unwrap_or_default();

    let line = WbOrdersLine {
        // Store the numeric WB order ID so the sticker API can use it later.
        // Statistics API will try to overwrite this with srid; service.rs preserves it.
        line_id: if order.id > 0 {
            order.id.to_string()
        } else {
            document_no.clone()
        },
        supplier_article: supplier_article.clone(),
        nm_id,
        barcode,
        category: None,
        subject: None,
        brand: None,
        tech_size: None,
        qty: 1.0,
        total_price: None,
        discount_percent: None,
        spp: None,
        finished_price: None,
        price_with_disc: None,
        price: order.price.map(|p| p as f64 / 100.0),
        sale_price: order.sale_price.filter(|&p| p > 0).map(|p| p as f64 / 100.0),
        dealer_price_ut: None,
        margin_pro: None,
    };

    let order_dt = order
        .created_at
        .as_ref()
        .and_then(|s| parse_wb_datetime(s))
        .unwrap_or_else(chrono::Utc::now);

    // Determine if cancelled based on status field
    let is_cancel = matches!(
        order.status.as_deref(),
        Some("cancelled") | Some("cancelledByClient") | Some("defect") | Some("didNotFit")
    );

    let state = WbOrdersState {
        order_dt,
        last_change_dt: None,
        is_cancel,
        cancel_dt: None,
        is_supply: Some(true), // marketplace API orders are always FBS
        is_realization: None,
    };

    let warehouse = WbOrdersWarehouse {
        warehouse_name: None,
        warehouse_type: None,
    };

    let geography = WbOrdersGeography {
        country_name: None,
        oblast_okrug_name: None,
        region_name: None,
    };

    // Parse income_id from supplyId ("WB-GI-32319994" → 32319994)
    let income_id = order
        .supply_id
        .as_ref()
        .and_then(|sid| sid.rsplit('-').next())
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|&v| v > 0);

    let source_meta = WbOrdersSourceMeta {
        income_id,
        sticker: None,
        g_number: None,
        raw_payload_ref: String::new(),
        marketplace_raw_payload_ref: None,
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    let description = format!(
        "WB Order {} - {}",
        supplier_article,
        order_dt.format("%Y-%m-%d %H:%M:%S")
    );

    // Marketplace API отдаёт createdAt в UTC (RFC3339 c Z); приводим document_date
    // к MSK из order_dt, чтобы формат совпадал с заказами из Statistics API.
    let document_date = Some(format_wb_local_datetime_seconds(&order_dt));

    let document = WbOrders::new_for_insert(
        document_no.clone(),
        description,
        header,
        line,
        state,
        warehouse,
        geography,
        source_meta,
        true,
        document_date,
    );

    // Store without raw JSON (marketplace API doesn't provide full analytics payload)
    let raw_json = serde_json::to_string(order)?;
    a015_wb_orders::service::store_document_with_raw(document, &raw_json).await?;
    let _ = a015_wb_orders::service::store_marketplace_raw_payload(&document_no, &raw_json).await?;

    Ok(true)
}
