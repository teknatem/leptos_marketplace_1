use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::domain::a010_ozon_fbs_posting;
use crate::domain::a011_ozon_fbo_posting;
use contracts::domain::a010_ozon_fbs_posting::aggregate::{
    OzonFbsPosting, OzonFbsPostingHeader, OzonFbsPostingLine, OzonFbsPostingSourceMeta,
    OzonFbsPostingState,
};
use contracts::domain::a011_ozon_fbo_posting::aggregate::{
    OzonFboPosting, OzonFboPostingHeader, OzonFboPostingLine, OzonFboPostingSourceMeta,
    OzonFboPostingState,
};
use super::super::ozon_api_client::OzonPosting;

/// Normalize OZON posting status
pub fn normalize_ozon_status(status: &str) -> String {
    match status.to_uppercase().as_str() {
        "DELIVERED" => "DELIVERED".to_string(),
        "CANCELLED" | "CANCELED" => "CANCELLED".to_string(),
        "" => "UNKNOWN".to_string(),
        other => other.to_uppercase(),
    }
}

pub async fn process_fbs_posting(
    connection: &ConnectionMP,
    organization_id: &str,
    posting: &OzonPosting,
) -> Result<bool> {
    let posting_number = posting.posting_number.clone();

    // Проверяем, существует ли документ
    let existing =
        a010_ozon_fbs_posting::service::get_by_document_no(&posting_number).await?;
    let is_new = existing.is_none();

    // Конвертируем продукты в строки документа
    let lines: Vec<OzonFbsPostingLine> = posting
        .products
        .iter()
        .enumerate()
        .map(|(idx, product)| OzonFbsPostingLine {
            line_id: format!("{}_{}", posting_number, idx + 1),
            product_id: product
                .product_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| product.offer_id.clone()),
            offer_id: product.offer_id.clone(),
            name: product.name.clone(),
            barcode: None,
            qty: product.quantity as f64,
            price_list: product.price,
            discount_total: None,
            price_effective: product.price,
            amount_line: product.price.map(|p| p * product.quantity as f64),
            currency_code: product.currency_code.clone(),
        })
        .collect();

    let delivered_at = posting
        .delivering_date
        .as_ref()
        .or(posting.delivered_at.as_ref())
        .and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

    // Создаем документ
    let header = OzonFbsPostingHeader {
        document_no: posting_number.clone(),
        scheme: "FBS".to_string(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let status_norm = normalize_ozon_status(&posting.status);
    let is_posted = true;

    let state = OzonFbsPostingState {
        status_raw: posting.status.clone(),
        status_norm,
        substatus_raw: posting.substatus.clone(),
        delivered_at,
        updated_at_source: None,
    };

    let source_meta = OzonFbsPostingSourceMeta {
        raw_payload_ref: String::new(),
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    let document = OzonFbsPosting::new_for_insert(
        posting_number.clone(),
        format!("FBS Posting {}", posting_number),
        header,
        lines,
        state,
        source_meta,
        is_posted,
    );

    let raw_json = serde_json::to_string(posting)?;
    a010_ozon_fbs_posting::service::store_document_with_raw(document, &raw_json).await?;
    
    Ok(is_new)
}

pub async fn process_fbo_posting(
    connection: &ConnectionMP,
    organization_id: &str,
    posting: &OzonPosting,
) -> Result<bool> {
    let posting_number = posting.posting_number.clone();

    // Проверяем, существует ли документ
    let existing =
        a011_ozon_fbo_posting::service::get_by_document_no(&posting_number).await?;
    let is_new = existing.is_none();

    // Конвертируем продукты в строки документа
    let lines: Vec<OzonFboPostingLine> = posting
        .products
        .iter()
        .enumerate()
        .map(|(idx, product)| OzonFboPostingLine {
            line_id: format!("{}_{}", posting_number, idx + 1),
            product_id: product
                .product_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| product.offer_id.clone()),
            offer_id: product.offer_id.clone(),
            name: product.name.clone(),
            barcode: None,
            qty: product.quantity as f64,
            price_list: product.price,
            discount_total: None,
            price_effective: product.price,
            amount_line: product.price.map(|p| p * product.quantity as f64),
            currency_code: product.currency_code.clone(),
        })
        .collect();

    let delivered_at = posting
        .delivered_at
        .as_ref()
        .or(posting.delivering_date.as_ref())
        .and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        });

    let created_at = posting.created_at.as_ref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok()
    });

    // Создаем документ
    let header = OzonFboPostingHeader {
        document_no: posting_number.clone(),
        scheme: "FBO".to_string(),
        connection_id: connection.base.id.as_string(),
        organization_id: organization_id.to_string(),
        marketplace_id: connection.marketplace_id.clone(),
    };

    let status_norm = normalize_ozon_status(&posting.status);
    let is_posted = true;

    let state = OzonFboPostingState {
        status_raw: posting.status.clone(),
        status_norm,
        substatus_raw: posting.substatus.clone(),
        created_at,
        delivered_at,
        updated_at_source: None,
    };

    let source_meta = OzonFboPostingSourceMeta {
        raw_payload_ref: String::new(),
        fetched_at: chrono::Utc::now(),
        document_version: 1,
    };

    let document = OzonFboPosting::new_for_insert(
        posting_number.clone(),
        format!("FBO Posting {}", posting_number),
        header,
        lines,
        state,
        source_meta,
        is_posted,
    );

    let raw_json = serde_json::to_string(posting)?;
    a011_ozon_fbo_posting::service::store_document_with_raw(document, &raw_json).await?;
    
    Ok(is_new)
}

