use axum::{extract::Query, Json};
use contracts::domain::a029_wb_supply::aggregate::{WbSupply, WbSupplyOrderRow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::a029_wb_supply;
use crate::shared::data::raw_storage;

/// Parse the numeric income_id from a WB supply ID like "WB-GI-32319994" → 32319994.
fn parse_income_id_from_supply_id(supply_id: &str) -> Option<i64> {
    supply_id
        .rsplit('-')
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|&v| v > 0)
}

/// Build WbSupplyOrderRow list from a015_wb_orders for supplies that have no stored orders.
async fn orders_from_a015(supply_id: &str) -> Vec<WbSupplyOrderRow> {
    let Some(income_id) = parse_income_id_from_supply_id(supply_id) else {
        return vec![];
    };
    match crate::domain::a015_wb_orders::service::list_by_income_id(income_id).await {
        Ok(orders) => orders
            .iter()
            .map(|o| {
                // line_id stores the numeric WB order ID if loaded via Marketplace API,
                // otherwise falls back to srid (non-numeric). Zero means sticker API won't work.
                let numeric_order_id = o.line.line_id.parse::<i64>().unwrap_or(0);

                // Parse sticker barcode from source_meta.sticker (shkID from Statistics API)
                // into part_a / part_b components if possible (format: "partA-partB")
                let (part_a, part_b) = match &o.source_meta.sticker {
                    Some(s) if !s.is_empty() => {
                        let mut parts = s.splitn(2, '-');
                        let a = parts.next().and_then(|v| v.parse::<i64>().ok());
                        let b = parts.next().and_then(|v| v.parse::<i64>().ok());
                        (a, b)
                    }
                    _ => (None, None),
                };

                WbSupplyOrderRow {
                    order_id: numeric_order_id,
                    order_uid: o.source_meta.g_number.clone(),
                    article: Some(o.line.supplier_article.clone()),
                    nm_id: Some(o.line.nm_id),
                    chrt_id: None,
                    barcodes: if o.line.barcode.is_empty() {
                        vec![]
                    } else {
                        vec![o.line.barcode.clone()]
                    },
                    price: o.line.price_with_disc.map(|p| (p * 100.0).round() as i64),
                    created_at: Some(o.state.order_dt.to_rfc3339()),
                    warehouse_id: None,
                    part_a,
                    part_b,
                    color_code: o.source_meta.sticker.clone(),
                    status: if o.state.is_cancel {
                        Some("cancel".to_string())
                    } else {
                        None
                    },
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!(
                "Failed to fetch a015 orders for supply {}: {}",
                supply_id,
                e
            );
            vec![]
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListSuppliesQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub organization_id: Option<String>,
    pub search_query: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub show_done: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WbSupplyListItemDto {
    pub id: String,
    pub supply_id: String,
    pub supply_name: Option<String>,
    pub is_deleted: bool,
    pub is_done: bool,
    pub is_b2b: bool,
    pub created_at_wb: Option<String>,
    pub closed_at_wb: Option<String>,
    pub cargo_type: Option<i32>,
    pub connection_id: String,
    pub organization_name: Option<String>,
    pub orders_count: i64,
    pub is_posted: bool,
}

#[derive(Debug, Serialize)]
pub struct PaginatedWbSupplyResponse {
    pub items: Vec<WbSupplyListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

pub async fn list_supplies(
    Query(query): Query<ListSuppliesQuery>,
) -> Result<Json<PaginatedWbSupplyResponse>, axum::http::StatusCode> {
    use a029_wb_supply::repository::{list_sql, WbSupplyListQuery};

    let page_size = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let page = if page_size > 0 { offset / page_size } else { 0 };
    let sort_by = query
        .sort_by
        .clone()
        .unwrap_or_else(|| "created_at_wb".to_string());
    let sort_desc = query.sort_desc.unwrap_or(true);
    let show_done = query.show_done.unwrap_or(true);

    let list_query = WbSupplyListQuery {
        date_from: query.date_from.clone(),
        date_to: query.date_to.clone(),
        connection_id: query.connection_id.clone(),
        organization_id: query.organization_id.clone(),
        search_query: query.search_query.clone(),
        sort_by,
        sort_desc,
        limit: page_size,
        offset,
        show_done,
    };

    let result = list_sql(list_query).await.map_err(|e| {
        tracing::error!("Failed to list WB supplies: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total = result.total;
    let total_pages = if page_size > 0 {
        (total + page_size - 1) / page_size
    } else {
        0
    };

    let items: Vec<WbSupplyListItemDto> = result
        .items
        .into_iter()
        .map(|row| WbSupplyListItemDto {
            id: row.id,
            supply_id: row.supply_id,
            supply_name: row.supply_name,
            is_deleted: row.is_deleted,
            is_done: row.is_done,
            is_b2b: row.is_b2b,
            created_at_wb: row.created_at_wb,
            closed_at_wb: row.closed_at_wb,
            cargo_type: row.cargo_type,
            connection_id: row.connection_id,
            organization_name: row.organization_name,
            orders_count: row.orders_count,
            is_posted: row.is_posted,
        })
        .collect();

    Ok(Json(PaginatedWbSupplyResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}

/// Resolve a supply by either its internal UUID or WB supply ID ("WB-GI-…").
/// Tabs now use WB-GI-... as key, so all per-supply endpoints must support both forms.
async fn resolve_supply(id: &str) -> Result<WbSupply, axum::http::StatusCode> {
    if id.starts_with("WB-") {
        a029_wb_supply::service::get_by_supply_id(id)
            .await
            .map_err(|e| {
                tracing::error!("resolve_supply by supply_id {}: {}", id, e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(axum::http::StatusCode::NOT_FOUND)
    } else {
        let uuid = Uuid::parse_str(id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
        a029_wb_supply::service::get_by_id(uuid)
            .await
            .map_err(|e| {
                tracing::error!("resolve_supply by uuid {}: {}", id, e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(axum::http::StatusCode::NOT_FOUND)
    }
}

pub async fn get_supply_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<WbSupply>, axum::http::StatusCode> {
    let mut item = resolve_supply(&id).await?;

    if item.supply_orders.is_empty() {
        item.supply_orders = orders_from_a015(&item.header.supply_id).await;
    }

    Ok(Json(item))
}

/// Look up a supply by its WB supply ID string (e.g. "WB-GI-32319994").
/// Returns the full aggregate — used when navigating from orders list where only WB ID is known.
/// If no stored orders, falls back to a015_wb_orders matched by income_id.
pub async fn get_supply_by_wb_id(
    axum::extract::Path(wb_id): axum::extract::Path<String>,
) -> Result<Json<WbSupply>, axum::http::StatusCode> {
    let mut item = a029_wb_supply::service::get_by_supply_id(&wb_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get WB supply by wb_id {}: {}", wb_id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    if item.supply_orders.is_empty() {
        item.supply_orders = orders_from_a015(&wb_id).await;
    }

    Ok(Json(item))
}

/// Returns the stored supply orders from the aggregate (no live WB call).
/// Falls back to a015_wb_orders by income_id if no orders are stored in the supply.
pub async fn get_supply_orders(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Vec<WbSupplyOrderRow>>, axum::http::StatusCode> {
    let supply = resolve_supply(&id).await?;

    let orders = if supply.supply_orders.is_empty() {
        orders_from_a015(&supply.header.supply_id).await
    } else {
        supply.supply_orders
    };

    Ok(Json(orders))
}

/// Proxy: get sticker images from WB API for selected orders in this supply
#[derive(Debug, Deserialize)]
pub struct StickersQuery {
    /// Sticker format: "png" | "svg" | "zplv" | "zplh". Query param: ?type=
    #[serde(rename = "type")]
    pub sticker_type: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// Comma-separated numeric WB order IDs to fetch stickers for.
    /// If omitted, fetches stickers for ALL orders in the supply.
    pub order_ids: Option<String>,
}

/// Single sticker row returned by the sticker endpoint.
/// `file` = base64-encoded image (PNG/SVG/ZPL) when available.
#[derive(Debug, Serialize, Deserialize)]
pub struct StickerItem {
    pub order_id: i64,
    pub article: Option<String>,
    pub shkid: Option<String>,
    pub part_a: Option<i64>,
    pub part_b: Option<i64>,
    pub barcode: Option<String>,
    pub file: Option<String>,
    /// True when this row comes from WB sticker API (has real partA/B + possibly image).
    /// False when this is a stub from stored order data.
    pub from_wb: bool,
}

/// Wrapper response so the frontend always receives structured diagnostics alongside sticker data.
#[derive(Debug, Serialize)]
pub struct StickersResponse {
    pub stickers: Vec<StickerItem>,
    pub orders_total: usize,
    pub orders_with_numeric_id: usize,
    /// Non-empty when something went wrong (API error, missing IDs, etc.)
    pub warning: Option<String>,
}

pub async fn get_supply_stickers(
    axum::extract::Path(id): axum::extract::Path<String>,
    Query(query): Query<StickersQuery>,
) -> Result<Json<StickersResponse>, axum::http::StatusCode> {
    let mut supply = resolve_supply(&id).await?;

    // Fall back to a015_wb_orders if no stored orders in supply aggregate
    if supply.supply_orders.is_empty() {
        supply.supply_orders = orders_from_a015(&supply.header.supply_id).await;
    }

    // If caller specified specific order IDs, filter to only those rows
    if let Some(ref ids_param) = query.order_ids {
        let requested: std::collections::HashSet<i64> = ids_param
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .filter(|&id| id > 0)
            .collect();
        if !requested.is_empty() {
            supply
                .supply_orders
                .retain(|o| requested.contains(&o.order_id));
        }
    }

    let orders_total = supply.supply_orders.len();

    if orders_total == 0 {
        return Ok(Json(StickersResponse {
            stickers: vec![],
            orders_total: 0,
            orders_with_numeric_id: 0,
            warning: Some(
                "Заказы не найдены. Загрузите «Оперативные заказы» и «Привязать к поставкам»."
                    .into(),
            ),
        }));
    }

    let sticker_type = query.sticker_type.as_deref().unwrap_or("png");
    let width = query.width.unwrap_or(58);
    let height = query.height.unwrap_or(40);

    // Numeric WB order IDs are stored in line_id by Marketplace API import
    let order_ids: Vec<i64> = supply
        .supply_orders
        .iter()
        .map(|o| o.order_id)
        .filter(|&id| id > 0)
        .collect();

    let orders_with_numeric_id = order_ids.len();

    // Stub: sticker numbers from stored order data, no images yet
    let stub: Vec<StickerItem> = supply
        .supply_orders
        .iter()
        .map(|o| StickerItem {
            order_id: o.order_id,
            article: o.article.clone(),
            shkid: o.color_code.clone(),
            part_a: o.part_a,
            part_b: o.part_b,
            barcode: o.barcodes.first().cloned(),
            file: None,
            from_wb: false,
        })
        .collect();

    if orders_with_numeric_id == 0 {
        return Ok(Json(StickersResponse {
            stickers: stub,
            orders_total,
            orders_with_numeric_id: 0,
            warning: Some(format!(
                "Все {} заказов не имеют numeric WB ID — изображения стикеров недоступны. \
                 Запустите «Оперативные заказы» (Marketplace API), затем нажмите «Загрузить из WB».",
                orders_total
            )),
        }));
    }

    let connection_id = &supply.header.connection_id;
    let connection_uuid = Uuid::parse_str(connection_id).ok();

    if let Some(conn_uuid) = connection_uuid {
        if let Ok(Some(connection)) =
            crate::domain::a006_connection_mp::service::get_by_id(conn_uuid).await
        {
            let api_client =
                crate::usecases::u504_import_from_wildberries::wildberries_api_client::WildberriesApiClient::new();

            match api_client
                .fetch_order_stickers(&connection, &order_ids, sticker_type, width, height)
                .await
            {
                Ok(wb_stickers) => {
                    // Merge WB sticker data into stub rows (preserving article / shkid info)
                    let mut result: Vec<StickerItem> = stub;
                    for wb in &wb_stickers {
                        if let Some(row) = result.iter_mut().find(|r| r.order_id == wb.order_id) {
                            row.part_a = wb.part_a;
                            row.part_b = wb.part_b;
                            row.barcode = wb.barcode.clone().or_else(|| row.barcode.clone());
                            row.file = wb.file.clone();
                            row.from_wb = true;
                        }
                    }
                    let warning = if orders_with_numeric_id < orders_total {
                        Some(format!(
                            "{} из {} заказов без numeric WB ID — для них изображения недоступны. \
                             Запустите «Оперативные заказы».",
                            orders_total - orders_with_numeric_id,
                            orders_total
                        ))
                    } else {
                        None
                    };
                    return Ok(Json(StickersResponse {
                        stickers: result,
                        orders_total,
                        orders_with_numeric_id,
                        warning,
                    }));
                }
                Err(e) => {
                    tracing::error!(
                        "WB stickers API error for supply {}: {}",
                        supply.header.supply_id,
                        e
                    );
                    return Ok(Json(StickersResponse {
                        stickers: stub,
                        orders_total,
                        orders_with_numeric_id,
                        warning: Some(format!("{}", e)),
                    }));
                }
            }
        }
    }

    Ok(Json(StickersResponse {
        stickers: stub,
        orders_total,
        orders_with_numeric_id,
        warning: Some("Не удалось подключиться к WB API (нет соединения).".into()),
    }))
}

pub async fn get_raw_json(
    axum::extract::Path(ref_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let raw_json_str = raw_storage::get_by_ref(&ref_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get raw JSON: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let json_value: serde_json::Value = serde_json::from_str(&raw_json_str).map_err(|e| {
        tracing::error!("Failed to parse raw JSON: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json_value))
}

pub async fn delete_supply(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a029_wb_supply::service::delete(uuid).await.map_err(|e| {
        tracing::error!("Failed to delete supply: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({"success": true})))
}
