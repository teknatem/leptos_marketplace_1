use super::super::wildberries_api_client::WbSaleRow;
use crate::domain::a012_wb_sales;
use crate::domain::a012_wb_sales::service::PostingPreparationCache;
use crate::shared::marketplaces::wildberries::datetime::parse_wb_datetime;
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a012_wb_sales::aggregate::{
    WbSales, WbSalesHeader, WbSalesLine, WbSalesSourceMeta, WbSalesState, WbSalesWarehouse,
};
use contracts::domain::common::AggregateId;
use std::collections::HashMap;
use uuid::Uuid;

/// Process a single WB sale row.
///
/// `existing_sale_ids` is a pre-loaded `HashMap<sale_id, existing_uuid>` for
/// the current import batch.  Providing it avoids an individual SELECT per row.
/// Pass an empty map to fall back to per-row DB lookups (legacy behaviour).
///
/// `cache` is shared across the whole batch so that repeated lookups for the
/// same products / organisations / prices are served from memory.
pub async fn process_sale_row(
    connection: &ConnectionMP,
    organization_id: &str,
    sale_row: &WbSaleRow,
    raw_json: &str,
    existing_sale_ids: &HashMap<String, Uuid>,
    cache: &mut PostingPreparationCache,
) -> Result<bool> {
    // SRID — уникальный идентификатор строки продажи
    let document_no = sale_row
        .srid
        .clone()
        .unwrap_or_else(|| format!("WB_{}", chrono::Utc::now().timestamp()));

    // sale_id — ГЛАВНЫЙ ключ дедупликации
    let sale_id = if let Some(sid) = sale_row.sale_id.clone() {
        sid
    } else {
        let supplier_article = sale_row.supplier_article.clone().unwrap_or_default();
        let event_type = if sale_row.quantity.unwrap_or(0) < 0 {
            "return"
        } else {
            "sale"
        };
        format!(
            "WB_GEN_{}_{}_{}_{}",
            document_no,
            event_type,
            supplier_article,
            chrono::Utc::now().timestamp_millis()
        )
    };

    // Lookup from the pre-loaded map (O(1)); fall back to DB only when the map
    // was not populated (e.g., called from non-batch code paths).
    let existing_uuid: Option<Uuid> = if existing_sale_ids.is_empty() {
        a012_wb_sales::service::get_by_sale_id(&sale_id)
            .await?
            .map(|doc| doc.base.id.value())
    } else {
        existing_sale_ids.get(&sale_id).copied()
    };
    let is_new = existing_uuid.is_none();

    let header = WbSalesHeader {
        document_no: document_no.clone(),
        sale_id: Some(sale_id.clone()),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let supplier_article = sale_row.supplier_article.clone().unwrap_or_default();

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
        is_fact: None,
        sell_out_plan: None,
        sell_out_fact: None,
        acquiring_fee_plan: None,
        acquiring_fee_fact: None,
        other_fee_plan: None,
        other_fee_fact: None,
        supplier_payout_plan: None,
        supplier_payout_fact: None,
        profit_plan: None,
        profit_fact: None,
        cost_of_production: None,
        commission_plan: None,
        commission_fact: None,
        dealer_price_ut: None,
    };

    let sale_dt = if let Some(date_str) = sale_row.sale_dt.as_ref() {
        parse_wb_datetime(date_str).unwrap_or_else(chrono::Utc::now)
    } else {
        chrono::Utc::now()
    };

    let last_change_dt = sale_row
        .last_change_date
        .as_ref()
        .and_then(|date_str| parse_wb_datetime(date_str));

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

    let mut document = WbSales::new_for_insert(
        document_no.clone(),
        format!("WB {} {}", event_type, supplier_article),
        header,
        line,
        state,
        warehouse,
        source_meta,
        true,
    );

    tracing::debug!(
        "Processing WB sale: sale_id={}, document_no={}, event_type={}, supplier_article={}",
        sale_id,
        document_no,
        event_type,
        supplier_article
    );

    match a012_wb_sales::service::store_document_with_raw_shared_cache(
        &mut document,
        raw_json,
        cache,
        existing_uuid,
    )
    .await
    {
        Ok((_id, _)) => {
            if is_new {
                tracing::debug!("Created new WB sale: sale_id={}", sale_id);
            } else {
                tracing::debug!("Updated existing WB sale: sale_id={}", sale_id);
            }
            Ok(is_new)
        }
        Err(e) => {
            tracing::error!(
                "Failed to store WB sale - sale_id: {}, error: {}",
                sale_id,
                e
            );

            if e.to_string().contains("UNIQUE constraint failed") {
                tracing::error!(
                    "UNIQUE constraint violation on sale_id: {}\n  \
                     This should not happen as sale_id is unique.\n  \
                     Possible cause: race condition during parallel import",
                    sale_id
                );
            }

            Err(e)
        }
    }
}
