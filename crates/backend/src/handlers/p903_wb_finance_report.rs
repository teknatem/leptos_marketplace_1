use axum::{extract::Query, Json};
use contracts::projections::p903_wb_finance_report::dto::{
    WbFinanceReportDetailResponse, WbFinanceReportDto, WbFinanceReportListRequest,
    WbFinanceReportListResponse,
};
use serde::Deserialize;

use crate::projections::p903_wb_finance_report::repository;

/// Handler для получения списка финансовых отчетов с фильтрами
pub async fn list_reports(
    Query(req): Query<WbFinanceReportListRequest>,
) -> Result<Json<WbFinanceReportListResponse>, axum::http::StatusCode> {
    let (items, total) = repository::list_with_filters(
        &req.date_from,
        &req.date_to,
        req.nm_id,
        req.sa_name,
        req.connection_mp_ref,
        req.organization_ref,
        req.supplier_oper_name,
        req.srid,
        &req.sort_by,
        req.sort_desc,
        req.limit,
        req.offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to list finance report: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dtos: Vec<WbFinanceReportDto> = items.into_iter().map(model_to_dto).collect();

    let has_more = total > (req.offset + dtos.len() as i32);

    Ok(Json(WbFinanceReportListResponse {
        items: dtos,
        total_count: total,
        has_more,
    }))
}

/// Handler для получения детальной информации по композитному ключу
pub async fn get_report_detail(
    axum::extract::Path((rr_dt, rrd_id)): axum::extract::Path<(String, i64)>,
) -> Result<Json<WbFinanceReportDetailResponse>, axum::http::StatusCode> {
    let item = repository::get_by_id(&rr_dt, rrd_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get finance report detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(WbFinanceReportDetailResponse {
        item: model_to_dto(item),
    }))
}

/// Handler для получения raw JSON по композитному ключу
pub async fn get_raw_json(
    axum::extract::Path((rr_dt, rrd_id)): axum::extract::Path<(String, i64)>,
) -> Result<String, axum::http::StatusCode> {
    let item = repository::get_by_id(&rr_dt, rrd_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get finance report raw json: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(item.extra.unwrap_or_else(|| "{}".to_string()))
}

/// Handler для поиска записей по srid
#[derive(Debug, Deserialize)]
pub struct SearchBySridQuery {
    pub srid: String,
}

pub async fn search_by_srid(
    Query(query): Query<SearchBySridQuery>,
) -> Result<Json<Vec<WbFinanceReportDto>>, axum::http::StatusCode> {
    let items = repository::search_by_srid(&query.srid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to search finance report by srid: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let dtos: Vec<WbFinanceReportDto> = items.into_iter().map(model_to_dto).collect();

    Ok(Json(dtos))
}

/// Преобразование Model в DTO
fn model_to_dto(model: repository::Model) -> WbFinanceReportDto {
    use chrono::DateTime;

    // Форматирование loaded_at_utc из RFC3339 в "YYYY-MM-DD HH:MM:SS"
    let loaded_at_formatted = DateTime::parse_from_rfc3339(&model.loaded_at_utc)
        .ok()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| model.loaded_at_utc.clone());

    WbFinanceReportDto {
        rr_dt: model.rr_dt,
        rrd_id: model.rrd_id,
        connection_mp_ref: model.connection_mp_ref,
        organization_ref: model.organization_ref,
        acquiring_fee: model.acquiring_fee,
        acquiring_percent: model.acquiring_percent,
        additional_payment: model.additional_payment,
        bonus_type_name: model.bonus_type_name,
        commission_percent: model.commission_percent,
        delivery_amount: model.delivery_amount,
        delivery_rub: model.delivery_rub,
        nm_id: model.nm_id,
        penalty: model.penalty,
        ppvz_vw: model.ppvz_vw,
        ppvz_vw_nds: model.ppvz_vw_nds,
        ppvz_sales_commission: model.ppvz_sales_commission,
        quantity: model.quantity,
        rebill_logistic_cost: model.rebill_logistic_cost,
        retail_amount: model.retail_amount,
        retail_price: model.retail_price,
        retail_price_withdisc_rub: model.retail_price_withdisc_rub,
        return_amount: model.return_amount,
        sa_name: model.sa_name,
        storage_fee: model.storage_fee,
        subject_name: model.subject_name,
        supplier_oper_name: model.supplier_oper_name,
        cashback_amount: model.cashback_amount,
        ppvz_for_pay: model.ppvz_for_pay,
        ppvz_kvw_prc: model.ppvz_kvw_prc,
        ppvz_kvw_prc_base: model.ppvz_kvw_prc_base,
        srv_dbs: model.srv_dbs,
        srid: model.srid,
        loaded_at_utc: loaded_at_formatted,
        payload_version: model.payload_version,
        extra: model.extra,
    }
}

