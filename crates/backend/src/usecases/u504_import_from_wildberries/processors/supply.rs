use super::super::wildberries_api_client::{WbStickerRow, WbSupplyRow};
use crate::domain::a029_wb_supply;
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::domain::a029_wb_supply::aggregate::{
    WbSupply, WbSupplyHeader, WbSupplyInfo, WbSupplyOrderRow, WbSupplySourceMeta,
};
use contracts::domain::common::AggregateId;
use std::collections::HashMap;

const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Clone)]
struct OrderNomenclatureLinks {
    nomenclature_ref: Option<String>,
    base_nomenclature_ref: Option<String>,
}

#[derive(Clone)]
struct StatOrderFallbackData {
    order_uid: Option<String>,
    article: Option<String>,
    nm_id: Option<i64>,
    barcodes: Vec<String>,
    price: Option<i64>,
    created_at: Option<String>,
    color_code: Option<String>,
    status: Option<String>,
    nomenclature_ref: Option<String>,
    base_nomenclature_ref: Option<String>,
}

fn sanitize_ref(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != ZERO_UUID)
        .map(ToOwned::to_owned)
}

fn build_order_links_index(
    orders: &[WbOrders],
) -> (
    HashMap<i64, OrderNomenclatureLinks>,
    HashMap<String, OrderNomenclatureLinks>,
) {
    let mut by_order_id = HashMap::new();
    let mut by_order_uid = HashMap::new();

    for order in orders {
        let links = OrderNomenclatureLinks {
            nomenclature_ref: sanitize_ref(order.nomenclature_ref.as_deref()),
            base_nomenclature_ref: sanitize_ref(order.base_nomenclature_ref.as_deref()),
        };

        if let Ok(order_id) = order.line.line_id.parse::<i64>() {
            if order_id > 0 {
                by_order_id.insert(order_id, links.clone());
            }
        }

        if let Some(order_uid) = order
            .source_meta
            .g_number
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            by_order_uid.insert(order_uid.to_string(), links);
        }
    }

    (by_order_id, by_order_uid)
}

fn build_stat_order_index(
    orders: &[WbOrders],
) -> (
    HashMap<i64, StatOrderFallbackData>,
    HashMap<String, StatOrderFallbackData>,
) {
    let mut by_order_id = HashMap::new();
    let mut by_order_uid = HashMap::new();

    for order in orders {
        let barcode = order.line.barcode.clone();
        let numeric_order_id = order.line.line_id.parse::<i64>().unwrap_or(0);
        let fallback = StatOrderFallbackData {
            order_uid: order.source_meta.g_number.clone(),
            article: Some(order.line.supplier_article.clone()),
            nm_id: Some(order.line.nm_id),
            barcodes: if barcode.is_empty() {
                vec![]
            } else {
                vec![barcode]
            },
            price: order
                .line
                .price_with_disc
                .map(|p| (p * 100.0).round() as i64),
            created_at: Some(order.state.order_dt.to_rfc3339()),
            color_code: order.source_meta.sticker.clone(),
            status: if order.state.is_cancel {
                Some("cancel".to_string())
            } else {
                None
            },
            nomenclature_ref: sanitize_ref(order.nomenclature_ref.as_deref()),
            base_nomenclature_ref: sanitize_ref(order.base_nomenclature_ref.as_deref()),
        };

        if numeric_order_id > 0 {
            by_order_id.insert(numeric_order_id, fallback.clone());
        }

        if let Some(order_uid) = order
            .source_meta
            .g_number
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            by_order_uid.insert(order_uid.to_string(), fallback);
        }
    }

    (by_order_id, by_order_uid)
}

fn apply_order_links(
    row: &mut WbSupplyOrderRow,
    by_order_id: &HashMap<i64, OrderNomenclatureLinks>,
    by_order_uid: &HashMap<String, OrderNomenclatureLinks>,
) {
    let links = if row.order_id > 0 {
        by_order_id.get(&row.order_id)
    } else {
        None
    }
    .or_else(|| {
        row.order_uid
            .as_ref()
            .and_then(|order_uid| by_order_uid.get(order_uid))
    });

    if let Some(links) = links {
        row.nomenclature_ref = links.nomenclature_ref.clone();
        row.base_nomenclature_ref = links.base_nomenclature_ref.clone();
    }
}

fn apply_stat_fallback(
    row: &mut WbSupplyOrderRow,
    by_order_id: &HashMap<i64, StatOrderFallbackData>,
    by_order_uid: &HashMap<String, StatOrderFallbackData>,
) {
    let fallback = if row.order_id > 0 {
        by_order_id.get(&row.order_id)
    } else {
        None
    }
    .or_else(|| {
        row.order_uid
            .as_ref()
            .and_then(|order_uid| by_order_uid.get(order_uid))
    });

    let Some(fallback) = fallback else {
        return;
    };

    if row.order_uid.is_none() {
        row.order_uid = fallback.order_uid.clone();
    }
    if row.article.is_none() {
        row.article = fallback.article.clone();
    }
    if row.nm_id.is_none() {
        row.nm_id = fallback.nm_id;
    }
    if row.barcodes.is_empty() {
        row.barcodes = fallback.barcodes.clone();
    }
    if row.price.is_none() {
        row.price = fallback.price;
    }
    if row.created_at.is_none() {
        row.created_at = fallback.created_at.clone();
    }
    if row.color_code.is_none() {
        row.color_code = fallback.color_code.clone();
    }
    if row.status.is_none() {
        row.status = fallback.status.clone();
    }
    if row.nomenclature_ref.is_none() {
        row.nomenclature_ref = fallback.nomenclature_ref.clone();
    }
    if row.base_nomenclature_ref.is_none() {
        row.base_nomenclature_ref = fallback.base_nomenclature_ref.clone();
    }
}

fn empty_order_row(order_id: i64) -> WbSupplyOrderRow {
    WbSupplyOrderRow {
        order_id,
        order_uid: None,
        article: None,
        nm_id: None,
        chrt_id: None,
        barcodes: vec![],
        price: None,
        created_at: None,
        warehouse_id: None,
        part_a: None,
        part_b: None,
        nomenclature_ref: None,
        base_nomenclature_ref: None,
        color_code: None,
        status: None,
    }
}

fn merge_stickers(rows: &mut [WbSupplyOrderRow], stickers: &[WbStickerRow]) {
    let sticker_map: HashMap<i64, &WbStickerRow> = stickers
        .iter()
        .filter(|sticker| sticker.order_id > 0)
        .map(|sticker| (sticker.order_id, sticker))
        .collect();

    for row in rows {
        let Some(sticker) = sticker_map.get(&row.order_id) else {
            continue;
        };
        row.part_a = sticker.part_a.or(row.part_a);
        row.part_b = sticker.part_b.or(row.part_b);
    }
}

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
        nomenclature_ref: sanitize_ref(order.nomenclature_ref.as_deref()),
        base_nomenclature_ref: sanitize_ref(order.base_nomenclature_ref.as_deref()),
        color_code: None,
        status,
    }
}

pub fn build_supply_rows_from_stat_orders(stat_orders: &[WbOrders]) -> Vec<WbSupplyOrderRow> {
    stat_orders
        .iter()
        .map(map_stat_order_to_supply_row)
        .collect()
}

pub async fn process_supply_row(
    connection: &ConnectionMP,
    organization_id: &str,
    supply_row: &WbSupplyRow,
    supply_order_ids: Vec<i64>,
    sticker_rows: Vec<WbStickerRow>,
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

    let (by_order_links_id, by_order_links_uid) = build_order_links_index(&stat_orders_fallback);
    let (by_stat_order_id, by_stat_order_uid) = build_stat_order_index(&stat_orders_fallback);

    let mut orders: Vec<WbSupplyOrderRow> = if !supply_order_ids.is_empty() {
        let mut rows: Vec<WbSupplyOrderRow> = Vec::with_capacity(supply_order_ids.len());
        let mut existing_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for order_id in supply_order_ids {
            if existing_ids.insert(order_id) {
                rows.push(empty_order_row(order_id));
            }
        }
        rows
    } else {
        build_supply_rows_from_stat_orders(&stat_orders_fallback)
    };

    for row in &mut orders {
        apply_stat_fallback(row, &by_stat_order_id, &by_stat_order_uid);
        apply_order_links(row, &by_order_links_id, &by_order_links_uid);
    }
    merge_stickers(&mut orders, &sticker_rows);

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
