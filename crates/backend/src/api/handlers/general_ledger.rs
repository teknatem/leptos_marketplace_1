use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};

use crate::shared::analytics::turnover_registry::get_turnover_class;
use contracts::projections::general_ledger::GeneralLedgerEntryDto;
use contracts::shared::analytics::TurnoverLayer;

#[derive(Debug, Deserialize)]
pub struct GeneralLedgerQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub registrator_ref: Option<String>,
    pub registrator_type: Option<String>,
    pub layer: Option<String>,
    pub turnover_code: Option<String>,
    pub debit_account: Option<String>,
    pub credit_account: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct GeneralLedgerListResponse {
    pub entries: Vec<GeneralLedgerEntryDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

pub async fn list(
    Query(q): Query<GeneralLedgerQuery>,
) -> Result<Json<GeneralLedgerListResponse>, axum::http::StatusCode> {
    let page_size = q.limit.unwrap_or(100) as usize;
    let offset = q.offset.unwrap_or(0) as usize;
    let page = if page_size > 0 { offset / page_size } else { 0 };
    let sort_desc = q.sort_desc.unwrap_or(true);

    let total = crate::projections::general_ledger::repository::count_with_filters(
        q.date_from.clone(),
        q.date_to.clone(),
        q.registrator_ref.clone(),
        q.registrator_type.clone(),
        q.layer.clone(),
        q.debit_account.clone(),
        q.credit_account.clone(),
        q.turnover_code.clone(),
    )
    .await
    .map_err(|e| {
        tracing::error!("general_ledger count error: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let rows = crate::projections::general_ledger::repository::list_with_filters(
        q.date_from,
        q.date_to,
        q.registrator_ref,
        q.registrator_type,
        q.layer,
        q.debit_account,
        q.credit_account,
        q.turnover_code,
        q.sort_by,
        sort_desc,
        Some(offset as u64),
        Some(page_size as u64),
    )
    .await
    .map_err(|e| {
        tracing::error!("general_ledger list error: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let entries: Vec<GeneralLedgerEntryDto> = rows.into_iter().map(to_dto).collect();

    let total = total as usize;
    let total_pages = if page_size > 0 {
        total.div_ceil(page_size)
    } else {
        0
    };

    Ok(Json(GeneralLedgerListResponse {
        entries,
        total,
        page,
        page_size,
        total_pages,
    }))
}

pub async fn get_by_id(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<GeneralLedgerEntryDto>, axum::http::StatusCode> {
    let item = crate::projections::general_ledger::repository::get_by_id(&id)
        .await
        .map_err(|e| {
            tracing::error!("general_ledger get_by_id error: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(to_dto(item)))
}

fn to_dto(row: crate::projections::general_ledger::repository::Model) -> GeneralLedgerEntryDto {
    let comment = get_turnover_class(&row.turnover_code)
        .map(|c| c.journal_comment.to_string())
        .unwrap_or_default();

    GeneralLedgerEntryDto {
        id: row.id,
        posting_id: row.posting_id,
        entry_date: row.entry_date,
        layer: TurnoverLayer::from_str(&row.layer).unwrap_or(TurnoverLayer::Oper),
        registrator_type: row.registrator_type,
        registrator_ref: row.registrator_ref,
        debit_account: row.debit_account,
        credit_account: row.credit_account,
        amount: row.amount,
        qty: row.qty,
        turnover_code: row.turnover_code,
        detail_kind: row.detail_kind,
        detail_id: row.detail_id,
        resource_name: row.resource_name,
        resource_sign: row.resource_sign,
        created_at: row.created_at,
        comment,
    }
}
