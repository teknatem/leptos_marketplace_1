use axum::{
    extract::{Path, Query},
    Json,
};
use contracts::projections::p907_ym_payment_report::dto::{
    YmPaymentReportDto, YmPaymentReportFilterOptionsResponse, YmPaymentReportListRequest,
    YmPaymentReportListResponse,
};
use serde::Deserialize;

use crate::projections::p907_ym_payment_report::repository;
use crate::usecases::u503_import_from_yandex::processors::payment_report as payment_report_processor;

/// Handler для получения списка записей отчёта по платежам YM
pub async fn list_reports(
    Query(req): Query<YmPaymentReportListRequest>,
) -> Result<Json<YmPaymentReportListResponse>, axum::http::StatusCode> {
    let (items, total) = repository::list_with_filters(
        &req.date_from,
        &req.date_to,
        req.transaction_type,
        req.payment_status,
        req.transaction_source,
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

#[derive(Debug, Deserialize)]
pub struct FilterOptionsQuery {
    #[serde(default)]
    pub date_from: String,
    #[serde(default)]
    pub date_to: String,
    pub connection_mp_ref: Option<String>,
    pub organization_ref: Option<String>,
}

pub async fn filter_options(
    Query(req): Query<FilterOptionsQuery>,
) -> Result<Json<YmPaymentReportFilterOptionsResponse>, axum::http::StatusCode> {
    let (transaction_types, payment_statuses, transaction_sources) =
        repository::list_filter_options(
            &req.date_from,
            &req.date_to,
            req.connection_mp_ref,
            req.organization_ref,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list YM payment report filter options: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(YmPaymentReportFilterOptionsResponse {
        transaction_types,
        payment_statuses,
        transaction_sources,
    }))
}

/// Handler для получения одной записи отчёта по платежам YM по UUID (`id` поле).
pub async fn get_report(
    Path(id): Path<String>,
) -> Result<Json<YmPaymentReportDto>, axum::http::StatusCode> {
    let item = repository::get_by_uuid(&id).await.map_err(|e| {
        tracing::error!("Failed to get YM payment report: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match item {
        Some(model) => Ok(Json(model_to_dto(model))),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

/// Migrate all SYNTH_... record keys to ymid_... format.
/// Safe to call multiple times — already-migrated rows are skipped.
pub async fn migrate_keys() -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let (migrated, _already_ymid, errors) = repository::migrate_synth_keys(|record| {
        payment_report_processor::build_ymid_key(
            record.order_id,
            record.transaction_date.as_deref().unwrap_or(""),
            record.transaction_type.as_deref().unwrap_or(""),
            record.shop_sku.as_deref().unwrap_or(""),
            record.transaction_sum,
        )
    })
    .await
    .map_err(|e| {
        tracing::error!("migrate_keys: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "migrated": migrated,
        "errors": errors,
        "message": format!("Migration complete: {} rows migrated, {} errors", migrated, errors)
    })))
}

fn model_to_dto(model: repository::Model) -> YmPaymentReportDto {
    YmPaymentReportDto {
        id: model.id,
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
