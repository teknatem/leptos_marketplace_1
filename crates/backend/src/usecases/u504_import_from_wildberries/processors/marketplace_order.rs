use super::super::wildberries_api_client::WbMarketplaceOrderRow;
use crate::domain::a015_wb_orders;
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

    if let Some(_existing) = existing {
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

    // Marketplace API price is in kopecks — convert to rubles for consistency
    let price_rubles = order.price.map(|p| p as f64 / 100.0);

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
        total_price: price_rubles,
        discount_percent: None,
        spp: None,
        finished_price: price_rubles,
        price_with_disc: price_rubles,
        dealer_price_ut: None,
        margin_pro: None,
    };

    let order_dt = order
        .created_at
        .as_ref()
        .and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        })
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
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    let description = format!(
        "WB Order {} - {}",
        supplier_article,
        order_dt.format("%Y-%m-%d %H:%M:%S")
    );

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
        order.created_at.clone(),
    );

    // Store without raw JSON (marketplace API doesn't provide full analytics payload)
    let raw_json = serde_json::to_string(order)?;
    a015_wb_orders::service::store_document_with_raw(document, &raw_json).await?;

    Ok(true)
}
