use axum::{
    extract::{Path, Query},
    Json,
};
use contracts::projections::p907_ym_payment_report::dto::{
    YmPaymentReportDto, YmPaymentReportListRequest, YmPaymentReportListResponse,
};

use crate::projections::p907_ym_payment_report::repository;

/// Handler для получения списка записей отчёта по платежам YM
pub async fn list_reports(
    Query(req): Query<YmPaymentReportListRequest>,
) -> Result<Json<YmPaymentReportListResponse>, axum::http::StatusCode> {
    let (items, total) = repository::list_with_filters(
        &req.date_from,
        &req.date_to,
        req.transaction_type,
        req.payment_status,
        req.shop_sku,
        req.order_id,
        req.connection_mp_ref,
        req.organization_ref,
        &req.sort_by,
        req.sort_desc,
        req.limit,
        req.offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to list YM payment report: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dtos: Vec<YmPaymentReportDto> = items.into_iter().map(model_to_dto).collect();
    let has_more = total > (req.offset + dtos.len() as i32);

    Ok(Json(YmPaymentReportListResponse {
        items: dtos,
        total_count: total,
        has_more,
    }))
}

/// Handler для получения одной записи отчёта по платежам YM по record_key
pub async fn get_report(
    Path(record_key): Path<String>,
) -> Result<Json<YmPaymentReportDto>, axum::http::StatusCode> {
    let item = repository::get_by_id(&record_key)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get YM payment report: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match item {
        Some(model) => Ok(Json(model_to_dto(model))),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

fn model_to_dto(model: repository::Model) -> YmPaymentReportDto {
    YmPaymentReportDto {
        record_key: model.record_key,
        transaction_id: model.transaction_id,
        connection_mp_ref: model.connection_mp_ref,
        organization_ref: model.organization_ref,
        business_id: model.business_id,
        partner_id: model.partner_id,
        shop_name: model.shop_name,
        inn: model.inn,
        model: model.model,
        transaction_date: model.transaction_date,
        transaction_type: model.transaction_type,
        transaction_source: model.transaction_source,
        transaction_sum: model.transaction_sum,
        payment_status: model.payment_status,
        order_id: model.order_id,
        shop_order_id: model.shop_order_id,
        order_creation_date: model.order_creation_date,
        order_delivery_date: model.order_delivery_date,
        order_type: model.order_type,
        shop_sku: model.shop_sku,
        offer_or_service_name: model.offer_or_service_name,
        count: model.count,
        act_id: model.act_id,
        act_date: model.act_date,
        bank_order_id: model.bank_order_id,
        bank_order_date: model.bank_order_date,
        bank_sum: model.bank_sum,
        claim_number: model.claim_number,
        bonus_account_year_month: model.bonus_account_year_month,
        comments: model.comments,
        loaded_at_utc: model.loaded_at_utc,
        payload_version: model.payload_version,
    }
}
