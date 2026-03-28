use axum::{extract::Query, Json};
use chrono::NaiveDate;
use contracts::projections::general_ledger::GeneralLedgerEntryDto;
use contracts::projections::p903_wb_finance_report::dto::{
    WbFinanceReportDetailResponse, WbFinanceReportDto, WbFinanceReportListRequest,
    WbFinanceReportListResponse,
};
use contracts::shared::analytics::TurnoverLayer;
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

    let gl_counts = crate::projections::general_ledger::repository::count_by_registrator_refs(
        &items
            .iter()
            .map(|item| item.id.clone())
            .collect::<Vec<_>>(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to count p903 general ledger rows: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dtos: Vec<WbFinanceReportDto> = items
        .into_iter()
        .map(|item| {
            let count = gl_counts.get(&item.id).copied().unwrap_or_default();
            model_to_dto(item, count)
        })
        .collect();

    let has_more = total > (req.offset + dtos.len() as i32);

    Ok(Json(WbFinanceReportListResponse {
        items: dtos,
        total_count: total,
        has_more,
    }))
}

/// Handler для получения детальной информации по композитному ключу
#[derive(Debug, Deserialize)]
pub struct OperationKindsQuery {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: Option<String>,
    pub organization_ref: Option<String>,
}

pub async fn list_operation_kinds(
    Query(query): Query<OperationKindsQuery>,
) -> Result<Json<Vec<String>>, axum::http::StatusCode> {
    let items = repository::list_distinct_supplier_oper_names(
        &query.date_from,
        &query.date_to,
        query.connection_mp_ref,
        query.organization_ref,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to list finance report operation kinds: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(items))
}

pub async fn get_report_detail_by_id(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<WbFinanceReportDetailResponse>, axum::http::StatusCode> {
    load_report_detail_by_id(&id).await.map(Json)
}

pub async fn post_report_by_id(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<WbFinanceReportDetailResponse>, axum::http::StatusCode> {
    let item = repository::get_by_id(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get finance report detail before post by id: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let day = NaiveDate::parse_from_str(&item.rr_dt, "%Y-%m-%d").map_err(|e| {
        tracing::error!("Failed to parse p903 rr_dt '{}' for post by id: {}", item.rr_dt, e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    crate::projections::p903_wb_finance_report::service::rebuild_day_from_existing(
        &item.connection_mp_ref,
        day,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to rebuild p903 general ledger for id {}: {}", id, e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    load_report_detail_by_id(&id).await.map(Json)
}

/// Handler для получения raw JSON по композитному ключу
pub async fn get_raw_json_by_id(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<String, axum::http::StatusCode> {
    let item = repository::get_by_id(&id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get finance report raw json by id: {}", e);
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
    let items = repository::search_by_srid(&query.srid).await.map_err(|e| {
        tracing::error!("Failed to search finance report by srid: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dtos: Vec<WbFinanceReportDto> = items
        .into_iter()
        .map(|item| model_to_dto(item, 0))
        .collect();

    Ok(Json(dtos))
}

/// Преобразование Model в DTO для списка (без extra для экономии трафика)
async fn load_report_detail_by_id(
    id: &str,
) -> Result<WbFinanceReportDetailResponse, axum::http::StatusCode> {
    let item = repository::get_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get finance report detail by id: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let general_ledger_entries =
        crate::projections::general_ledger::repository::list_by_registrator_ref(&item.id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to load p903 general ledger rows by id: {}", e);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?
            .into_iter()
            .map(to_general_ledger_dto)
            .collect::<Vec<_>>();

    Ok(WbFinanceReportDetailResponse {
        item: model_to_dto(item, general_ledger_entries.len()),
        general_ledger_entries,
    })
}

fn model_to_dto(
    model: repository::Model,
    general_ledger_entries_count: usize,
) -> WbFinanceReportDto {
    // Убрано форматирование даты для оптимизации - отправляем как есть
    // Поле extra не включаем в список - оно может содержать большой JSON (~3KB на запись)
    WbFinanceReportDto {
        id: model.id,
        rr_dt: model.rr_dt,
        rrd_id: model.rrd_id,
        source_row_ref: model.source_row_ref,
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
        loaded_at_utc: model.loaded_at_utc,
        payload_version: model.payload_version,
        general_ledger_entries_count,
        extra: None, // Исключаем из списка для экономии трафика (60MB -> ~5MB)
    }
}

fn to_general_ledger_dto(
    row: crate::projections::general_ledger::repository::Model,
) -> GeneralLedgerEntryDto {
    let comment =
        crate::shared::analytics::turnover_registry::get_turnover_class(&row.turnover_code)
            .map(|c| c.journal_comment.to_string())
            .unwrap_or_default();

    GeneralLedgerEntryDto {
        id: row.id,
        entry_date: row.entry_date,
        layer: TurnoverLayer::from_str(&row.layer).unwrap_or(TurnoverLayer::Oper),
        cabinet_mp: row.cabinet_mp,
        registrator_type: row.registrator_type,
        registrator_ref: row.registrator_ref,
        debit_account: row.debit_account,
        credit_account: row.credit_account,
        amount: row.amount,
        qty: row.qty,
        turnover_code: row.turnover_code,
        resource_table: row.resource_table,
        resource_field: row.resource_field,
        resource_sign: row.resource_sign,
        created_at: row.created_at,
        comment,
    }
}
