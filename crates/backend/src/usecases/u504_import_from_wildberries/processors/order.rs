use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::a015_wb_orders;
use contracts::domain::a015_wb_orders::aggregate::{
    WbOrders, WbOrdersGeography, WbOrdersHeader, WbOrdersLine, WbOrdersSourceMeta,
    WbOrdersState, WbOrdersWarehouse,
};
use super::super::wildberries_api_client::WbOrderRow;

pub async fn process_order_row(
    connection: &ConnectionMP,
    organization_id: &str,
    order_row: &WbOrderRow,
) -> Result<bool> {
    let document_no = order_row
        .srid
        .clone()
        .unwrap_or_else(|| format!("WB_ORDER_{}", chrono::Utc::now().timestamp()));

    // Проверяем, существует ли документ
    let existing = a015_wb_orders::service::get_by_document_no(&document_no).await?;
    let is_new = existing.is_none();

    // Создаем header
    let header = WbOrdersHeader {
        document_no: document_no.clone(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let supplier_article = order_row.supplier_article.clone().unwrap_or_default();

    // Создаем line
    let line = WbOrdersLine {
        line_id: order_row.srid.clone().unwrap_or_else(|| document_no.clone()),
        supplier_article: supplier_article.clone(),
        nm_id: order_row.nm_id.unwrap_or(0),
        barcode: order_row.barcode.clone().unwrap_or_default(),
        category: order_row.category.clone(),
        subject: order_row.subject.clone(),
        brand: order_row.brand.clone(),
        tech_size: order_row.tech_size.clone(),
        qty: 1.0,
        total_price: order_row.total_price,
        discount_percent: order_row.discount_percent,
        spp: order_row.spp,
        finished_price: order_row.finished_price,
        price_with_disc: order_row.price_with_disc,
    };

    // Парсим даты
    let order_dt = if let Some(date_str) = order_row.date.as_ref() {
        chrono::DateTime::parse_from_rfc3339(date_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .map(|ndt| chrono::DateTime::from_naive_utc_and_offset(ndt, chrono::Utc))
            })
            .unwrap_or_else(chrono::Utc::now)
    } else {
        chrono::Utc::now()
    };

    let last_change_dt = order_row.last_change_date.as_ref().and_then(|date_str| {
        chrono::DateTime::parse_from_rfc3339(date_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .map(|ndt| chrono::DateTime::from_naive_utc_and_offset(ndt, chrono::Utc))
            })
    });

    let cancel_dt = order_row.cancel_date.as_ref().and_then(|date_str| {
        chrono::DateTime::parse_from_rfc3339(date_str)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .map(|ndt| chrono::DateTime::from_naive_utc_and_offset(ndt, chrono::Utc))
            })
    });

    let state = WbOrdersState {
        order_dt,
        last_change_dt,
        is_cancel: order_row.is_cancel.unwrap_or(false),
        cancel_dt,
        is_supply: order_row.is_supply,
        is_realization: order_row.is_realization,
    };

    let warehouse = WbOrdersWarehouse {
        warehouse_name: order_row.warehouse_name.clone(),
        warehouse_type: order_row.warehouse_type.clone(),
    };

    let geography = WbOrdersGeography {
        country_name: order_row.country_name.clone(),
        oblast_okrug_name: order_row.oblast_okrug_name.clone(),
        region_name: order_row.region_name.clone(),
    };

    let source_meta = WbOrdersSourceMeta {
        income_id: order_row.income_id,
        sticker: order_row.sticker.clone(),
        g_number: order_row.g_number.clone(),
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
        order_row.date.clone(),
    );

    let raw_json = serde_json::to_string(order_row)?;
    a015_wb_orders::service::store_document_with_raw(document, &raw_json).await?;
    
    Ok(is_new)
}

