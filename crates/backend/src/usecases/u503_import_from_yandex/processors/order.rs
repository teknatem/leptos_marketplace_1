use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::a013_ym_order;
use contracts::domain::a013_ym_order::aggregate::{
    YmOrder, YmOrderHeader, YmOrderLine, YmOrderSourceMeta, YmOrderState,
};
use super::super::yandex_api_client::YmOrderItem;

/// Normalize Yandex Market order status
pub fn normalize_ym_status(status: &str) -> String {
    match status.to_uppercase().as_str() {
        "DELIVERED" | "PICKUP" => "DELIVERED".to_string(),
        "CANCELLED" | "CANCELLED_IN_DELIVERY" | "CANCELLED_BEFORE_PROCESSING" => {
            "CANCELLED".to_string()
        }
        "PROCESSING" | "PENDING" | "RESERVATION" => "PROCESSING".to_string(),
        "DELIVERY" => "IN_DELIVERY".to_string(),
        "" => "UNKNOWN".to_string(),
        other => other.to_uppercase(),
    }
}

/// Parse Yandex Market date (supports multiple formats)
pub fn parse_ym_date(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Try RFC3339 first (e.g., "2024-01-15T10:30:00Z")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&chrono::Utc));
    }

    // Try format "DD-MM-YYYY HH:MM:SS" (Yandex Market format with time)
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%d-%m-%Y %H:%M:%S") {
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }

    // Try format "DD-MM-YYYY" (Yandex Market format without time)
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%d-%m-%Y") {
        let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive_datetime,
            chrono::Utc,
        ));
    }

    // Try format "YYYY-MM-DD HH:MM:SS"
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }

    // Try format "YYYY-MM-DD"
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
        return Some(chrono::DateTime::from_naive_utc_and_offset(
            naive_datetime,
            chrono::Utc,
        ));
    }

    None
}

pub async fn process_order(
    connection: &ConnectionMP,
    organization_id: &str,
    order_details: &YmOrderItem,
) -> Result<bool> {
    let order_id_str = order_details.id.to_string();

    // Check if exists
    let existing = a013_ym_order::service::get_by_document_no(&order_id_str).await?;
    let is_new = existing.is_none();

    // Map lines
    let lines: Vec<YmOrderLine> = order_details
        .items
        .iter()
        .map(|item| {
            let price_list = item.price;
            let discount = item.subsidy.unwrap_or(0.0);
            let price_effective = price_list.map(|p| p - discount);
            let amount_line = price_list.map(|p| p * item.count as f64);
            let subsidies_json = item
                .subsidies
                .as_ref()
                .and_then(|s| serde_json::to_string(s).ok());

            YmOrderLine {
                line_id: item.id.to_string(),
                shop_sku: item.shop_sku.clone().unwrap_or_default(),
                offer_id: item.offer_id.clone().unwrap_or_default(),
                name: item.name.clone().unwrap_or_default(),
                qty: item.count as f64,
                price_list,
                discount_total: item.subsidy,
                price_effective,
                amount_line,
                currency_code: order_details.currency.clone(),
                buyer_price: item.buyer_price,
                subsidies_json,
                status: item.status.clone(),
                price_plan: Some(0.0),
                marketplace_product_ref: None,
                nomenclature_ref: None,
            }
        })
        .collect();

    // Parse dates
    let status_changed_at = order_details
        .status_update_date
        .as_ref()
        .and_then(|s| parse_ym_date(s));

    let delivery_date = order_details
        .delivery
        .as_ref()
        .and_then(|d| d.dates.as_ref())
        .and_then(|dates| dates.real_delivery_date.as_ref())
        .and_then(|s| parse_ym_date(s));

    let creation_date = order_details
        .creation_date
        .as_ref()
        .and_then(|s| parse_ym_date(s));

    let status_raw = order_details
        .status
        .clone()
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let status_norm = normalize_ym_status(&status_raw);

    let subsidies_json = order_details
        .subsidies
        .as_ref()
        .and_then(|s| serde_json::to_string(s).ok());

    // Create aggregate
    let header = YmOrderHeader {
        document_no: order_id_str.clone(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
        campaign_id: connection
            .supplier_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        total_amount: order_details.total,
        currency: order_details.currency.clone(),
        items_total: order_details.items_total,
        delivery_total: order_details.delivery_total,
        subsidies_json,
    };

    let state = YmOrderState {
        status_raw,
        substatus_raw: order_details.substatus.clone(),
        status_norm,
        status_changed_at,
        updated_at_source: status_changed_at,
        creation_date,
        delivery_date,
    };

    let source_meta = YmOrderSourceMeta {
        raw_payload_ref: String::new(),
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    let document = YmOrder::new_for_insert(
        order_id_str.clone(),
        format!("YM Order {}", order_id_str),
        header,
        lines,
        state,
        source_meta,
        true,
    );

    let raw_json = serde_json::to_string(&order_details)?;
    a013_ym_order::service::store_document_with_raw(document, &raw_json).await?;
    
    Ok(is_new)
}

