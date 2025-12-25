use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::a012_wb_sales;
use contracts::domain::a012_wb_sales::aggregate::{
    WbSales, WbSalesHeader, WbSalesLine, WbSalesSourceMeta, WbSalesState, WbSalesWarehouse,
};
use super::super::wildberries_api_client::WbSaleRow;

pub async fn process_sale_row(
    connection: &ConnectionMP,
    organization_id: &str,
    sale_row: &WbSaleRow,
) -> Result<bool> {
    // SRID - уникальный идентификатор строки продажи
    let document_no = sale_row
        .srid
        .clone()
        .unwrap_or_else(|| format!("WB_{}", chrono::Utc::now().timestamp()));

    let sale_id = sale_row.sale_id.clone();

    // Проверяем, существует ли документ
    let existing = if let Some(ref sid) = sale_id {
        a012_wb_sales::service::get_by_sale_id(sid).await?
    } else {
        a012_wb_sales::service::get_by_document_no(&document_no).await?
    };
    let is_new = existing.is_none();

    // Создаем header
    let header = WbSalesHeader {
        document_no: document_no.clone(),
        sale_id: sale_id.clone(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let supplier_article = sale_row.supplier_article.clone().unwrap_or_default();

    // Создаем line
    let line = WbSalesLine {
        line_id: sale_row.srid.clone().unwrap_or_else(|| document_no.clone()),
        supplier_article: supplier_article.clone(),
        nm_id: sale_row.nm_id.unwrap_or(0),
        barcode: sale_row.barcode.clone().unwrap_or_default(),
        name: sale_row
            .brand
            .clone()
            .unwrap_or_else(|| "Unknown".to_string()),
        qty: sale_row.quantity.unwrap_or(1) as f64,
        price_list: sale_row.price_with_disc,
        discount_total: sale_row.discount,
        price_effective: sale_row.price_with_disc,
        amount_line: sale_row.for_pay,
        currency_code: Some("RUB".to_string()),
        total_price: sale_row.total_price,
        payment_sale_amount: sale_row.payment_sale_amount,
        discount_percent: sale_row.discount_percent,
        spp: sale_row.spp,
        finished_price: sale_row.finished_price,
    };

    // Парсим даты
    let sale_dt = if let Some(date_str) = sale_row.sale_dt.as_ref() {
        chrono::DateTime::parse_from_rfc3339(date_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S").map(
                    |ndt| chrono::DateTime::from_naive_utc_and_offset(ndt, chrono::Utc),
                )
            })
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").map(
                    |ndt| chrono::DateTime::from_naive_utc_and_offset(ndt, chrono::Utc),
                )
            })
            .unwrap_or_else(|_| chrono::Utc::now())
    } else {
        chrono::Utc::now()
    };

    let last_change_dt = sale_row.last_change_date.as_ref().and_then(|date_str| {
        chrono::DateTime::parse_from_rfc3339(date_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").map(
                    |ndt| chrono::DateTime::from_naive_utc_and_offset(ndt, chrono::Utc),
                )
            })
            .ok()
    });

    let event_type = if sale_row.quantity.unwrap_or(0) < 0 {
        "return".to_string()
    } else {
        "sale".to_string()
    };

    let state = WbSalesState {
        event_type: event_type.clone(),
        status_norm: if event_type == "sale" {
            "DELIVERED".to_string()
        } else {
            "RETURNED".to_string()
        },
        sale_dt,
        last_change_dt,
        is_supply: sale_row.is_supply,
        is_realization: sale_row.is_realization,
    };

    let warehouse = WbSalesWarehouse {
        warehouse_name: sale_row.warehouse_name.clone(),
        warehouse_type: sale_row.warehouse_type.clone(),
    };

    let source_meta = WbSalesSourceMeta {
        raw_payload_ref: String::new(),
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    let document = WbSales::new_for_insert(
        document_no.clone(),
        format!("WB {} {}", event_type, supplier_article),
        header,
        line,
        state,
        warehouse,
        source_meta,
        true,
    );

    let raw_json = serde_json::to_string(sale_row)?;
    a012_wb_sales::service::store_document_with_raw(document, &raw_json).await?;
    
    Ok(is_new)
}

