use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};

use crate::general_ledger::drilldown_dimensions::dimensions_for_turnover;
use crate::general_ledger::drilldown_session_repository;
use crate::general_ledger::report_repository;
use crate::general_ledger::turnover_registry::{get_turnover_class, TURNOVER_CLASSES};
use contracts::general_ledger::{
    GeneralLedgerEntryDto, GeneralLedgerTurnoverDto, GlAccountViewQuery, GlAccountViewResponse,
    GlDimensionsResponse, GlDrilldownQuery, GlDrilldownResponse, GlDrilldownSessionCreate,
    GlDrilldownSessionCreateResponse, GlDrilldownSessionRecord, GlReportQuery, GlReportResponse,
    WbWeeklyReconciliationQuery, WbWeeklyReconciliationResponse,
};
use contracts::shared::analytics::TurnoverLayer;

#[derive(Debug, Deserialize)]
pub struct GeneralLedgerQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub registrator_ref: Option<String>,
    pub registrator_type: Option<String>,
    pub layer: Option<String>,
    pub turnover_code: Option<String>,
    pub connection_mp_ref: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct GeneralLedgerTurnoverListResponse {
    pub items: Vec<GeneralLedgerTurnoverDto>,
    pub total: usize,
}

pub async fn list(
    Query(q): Query<GeneralLedgerQuery>,
) -> Result<Json<GeneralLedgerListResponse>, axum::http::StatusCode> {
    let page_size = q.limit.unwrap_or(100) as usize;
    let offset = q.offset.unwrap_or(0) as usize;
    let page = if page_size > 0 { offset / page_size } else { 0 };
    let sort_desc = q.sort_desc.unwrap_or(true);

    let total = crate::general_ledger::repository::count_with_filters(
        q.date_from.clone(),
        q.date_to.clone(),
        q.registrator_ref.clone(),
        q.registrator_type.clone(),
        q.layer.clone(),
        q.debit_account.clone(),
        q.credit_account.clone(),
        q.turnover_code.clone(),
        q.connection_mp_ref.clone(),
    )
    .await
    .map_err(|e| {
        tracing::error!("general_ledger count error: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let rows = crate::general_ledger::repository::list_with_filters(
        q.date_from,
        q.date_to,
        q.registrator_ref,
        q.registrator_type,
        q.layer,
        q.debit_account,
        q.credit_account,
        q.turnover_code,
        q.connection_mp_ref,
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
    let item = crate::general_ledger::repository::get_by_id(&id)
        .await
        .map_err(|e| {
            tracing::error!("general_ledger get_by_id error: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(to_dto(item)))
}

pub async fn list_turnovers(
) -> Result<Json<GeneralLedgerTurnoverListResponse>, axum::http::StatusCode> {
    let counts = crate::general_ledger::repository::count_grouped_by_turnover_code()
        .await
        .map_err(|e| {
            tracing::error!("general_ledger turnover counts error: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut items = TURNOVER_CLASSES
        .iter()
        .map(|item| GeneralLedgerTurnoverDto {
            code: item.code.to_string(),
            name: item.name.to_string(),
            description: item.description.to_string(),
            llm_description: item.llm_description.to_string(),
            scope: item.scope,
            value_kind: item.value_kind,
            agg_kind: item.agg_kind,
            selection_rule: item.selection_rule,
            sign_policy: item.sign_policy,
            report_group: item.report_group,
            aliases: item.aliases.iter().map(|value| value.to_string()).collect(),
            source_examples: item
                .source_examples
                .iter()
                .map(|value| value.to_string())
                .collect(),
            formula_hint: item.formula_hint.to_string(),
            notes: item.notes.to_string(),
            debit_account: item.debit_account.to_string(),
            credit_account: item.credit_account.to_string(),
            generates_journal_entry: item.generates_journal_entry,
            journal_comment: item.journal_comment.to_string(),
            gl_entries_count: counts.get(item.code).copied().unwrap_or(0),
            available_dimensions: dimensions_for_turnover(item.code),
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .gl_entries_count
            .cmp(&left.gl_entries_count)
            .then_with(|| left.code.cmp(&right.code))
    });

    let total = items.len();

    Ok(Json(GeneralLedgerTurnoverListResponse { items, total }))
}

// ─────────────────────────────────────────────────────────────────────────────
// GL Report endpoints
// ─────────────────────────────────────────────────────────────────────────────

pub async fn report(
    Json(query): Json<GlReportQuery>,
) -> Result<Json<GlReportResponse>, axum::http::StatusCode> {
    report_repository::get_report(&query)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("GL report error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

#[derive(Debug, serde::Deserialize)]
pub struct DimensionsParams {
    pub turnover_code: String,
}

pub async fn report_dimensions(
    Query(params): Query<DimensionsParams>,
) -> Json<GlDimensionsResponse> {
    let dimensions = dimensions_for_turnover(&params.turnover_code);
    Json(GlDimensionsResponse {
        turnover_code: params.turnover_code,
        dimensions,
    })
}

pub async fn report_drilldown(
    Json(query): Json<GlDrilldownQuery>,
) -> Result<Json<GlDrilldownResponse>, axum::http::StatusCode> {
    report_repository::get_drilldown(&query)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("GL drilldown error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn create_drilldown_session(
    Json(body): Json<GlDrilldownSessionCreate>,
) -> Result<Json<GlDrilldownSessionCreateResponse>, axum::http::StatusCode> {
    drilldown_session_repository::create_session(&body)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("GL drilldown session create error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn get_drilldown_session(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<GlDrilldownSessionRecord>, axum::http::StatusCode> {
    let session = drilldown_session_repository::get_session(&id)
        .await
        .map_err(|e| {
            tracing::error!("GL drilldown session get error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    drilldown_session_repository::touch_session(&id)
        .await
        .map_err(|e| {
            tracing::error!("GL drilldown session touch error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(session))
}

pub async fn get_drilldown_session_data(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<GlDrilldownResponse>, axum::http::StatusCode> {
    let session = drilldown_session_repository::get_session(&id)
        .await
        .map_err(|e| {
            tracing::error!("GL drilldown session data get error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    drilldown_session_repository::touch_session(&id)
        .await
        .map_err(|e| {
            tracing::error!("GL drilldown session data touch error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    report_repository::get_drilldown(&session.query)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("GL drilldown session data error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// GL Account View endpoint
// ─────────────────────────────────────────────────────────────────────────────

pub async fn account_view(
    Json(query): Json<GlAccountViewQuery>,
) -> Result<Json<GlAccountViewResponse>, axum::http::StatusCode> {
    crate::general_ledger::account_view::repository::get_view(&query)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("GL account view error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn wb_weekly_reconciliation(
    Query(query): Query<WbWeeklyReconciliationQuery>,
) -> Result<Json<WbWeeklyReconciliationResponse>, axum::http::StatusCode> {
    crate::general_ledger::weekly_reconciliation::get_report(&query)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("WB weekly reconciliation report error: {e}");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

fn to_dto(row: crate::general_ledger::repository::Model) -> GeneralLedgerEntryDto {
    let comment = get_turnover_class(&row.turnover_code)
        .map(|c| c.journal_comment.to_string())
        .unwrap_or_default();

    GeneralLedgerEntryDto {
        id: row.id,
        entry_date: row.entry_date,
        layer: TurnoverLayer::from_str(&row.layer).unwrap_or(TurnoverLayer::Oper),
        connection_mp_ref: row.connection_mp_ref,
        registrator_type: row.registrator_type,
        registrator_ref: row.registrator_ref,
        order_id: row.order_id,
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
