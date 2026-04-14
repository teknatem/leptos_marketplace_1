use super::super::wildberries_api_client::{WbSupplyOrderApiRow, WbSupplyRow};
use crate::domain::a029_wb_supply;
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::domain::a029_wb_supply::aggregate::{
    WbSupply, WbSupplyHeader, WbSupplyInfo, WbSupplyOrderRow, WbSupplySourceMeta,
};
use contracts::domain::common::AggregateId;

fn parse_wb_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|ndt| chrono::DateTime::from_naive_utc_and_offset(ndt, chrono::Utc))
        })
}

fn map_order_row(row: &WbSupplyOrderApiRow) -> WbSupplyOrderRow {
    WbSupplyOrderRow {
        order_id: row.id,
        order_uid: row.order_uid.clone(),
        article: row.article.clone(),
        nm_id: row.nm_id,
        chrt_id: row.chrt_id,
        barcodes: row.barcodes.clone().unwrap_or_default(),
        price: row.price,
        created_at: row.created_at.clone(),
        warehouse_id: row.warehouse_id,
        part_a: row.part_a,
        part_b: row.part_b,
        color_code: row.color_code.clone(),
        status: row.status.clone(),
    }
}

/// Map a WbOrders (from statistics API / a015) to WbSupplyOrderRow.
/// Used as fallback when the marketplace API doesn't return orders for a supply.
fn map_stat_order_to_supply_row(order: &WbOrders) -> WbSupplyOrderRow {
    let barcode = order.line.barcode.clone();
    let numeric_order_id = order.line.line_id.parse::<i64>().unwrap_or(0);
    let price_kopecks = order
        .line
        .price_with_disc
        .map(|p| (p * 100.0).round() as i64);
    let status = if order.state.is_cancel {
        Some("cancel".to_string())
    } else {
        None
    };
    WbSupplyOrderRow {
        order_id: numeric_order_id,
        order_uid: order.source_meta.g_number.clone(),
        article: Some(order.line.supplier_article.clone()),
        nm_id: Some(order.line.nm_id),
        chrt_id: None,
        barcodes: if barcode.is_empty() {
            vec![]
        } else {
            vec![barcode]
        },
        price: price_kopecks,
        created_at: Some(order.state.order_dt.to_rfc3339()),
        warehouse_id: None,
        part_a: None,
        part_b: None,
        color_code: None,
        status,
    }
}

pub async fn process_supply_row(
    connection: &ConnectionMP,
    organization_id: &str,
    supply_row: &WbSupplyRow,
    supply_orders: Vec<WbSupplyOrderApiRow>,
    stat_orders_fallback: Vec<WbOrders>,
) -> Result<bool> {
    let existing = a029_wb_supply::service::get_by_supply_id(&supply_row.id).await?;
    let is_new = existing.is_none();

    let header = WbSupplyHeader {
        supply_id: supply_row.id.clone(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let info = WbSupplyInfo {
        name: supply_row.name.clone(),
        is_b2b: supply_row.is_b2b.unwrap_or(false),
        is_done: supply_row.done.unwrap_or(false),
        created_at_wb: supply_row.created_at.as_deref().and_then(parse_wb_datetime),
        closed_at_wb: supply_row.closed_at.as_deref().and_then(parse_wb_datetime),
        scan_dt: supply_row.scan_dt.as_deref().and_then(parse_wb_datetime),
        cargo_type: supply_row.cargo_type,
        cross_border_type: supply_row.cross_border_type,
        destination_office_id: supply_row.destination_office_id,
    };

    let source_meta = WbSupplySourceMeta {
        raw_payload_ref: String::new(),
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    // Use marketplace API orders if available; fall back to statistics API orders
    let orders: Vec<WbSupplyOrderRow> = if !supply_orders.is_empty() {
        supply_orders.iter().map(map_order_row).collect()
    } else {
        stat_orders_fallback
            .iter()
            .map(map_stat_order_to_supply_row)
            .collect()
    };
    let orders_count = orders.len();

    let created_at_str = supply_row.created_at.clone();
    let description = format!("WB Supply {} - {} заказов", supply_row.id, orders_count);

    let document = WbSupply::new_for_insert(
        supply_row.id.clone(),
        description,
        header,
        info,
        source_meta,
        false,
        orders,
        created_at_str,
    );

    let raw_json = serde_json::to_string(supply_row)?;
    a029_wb_supply::service::store_document_with_raw(document, &raw_json).await?;

    Ok(is_new)
}
