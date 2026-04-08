use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use contracts::general_ledger::GeneralLedgerEntryDto;
use contracts::projections::p911_wb_advert_by_items::dto::{
    WbAdvertByItemDetailDto, WbAdvertByItemDto, WbAdvertByItemListResponse,
};
use contracts::shared::analytics::{AggKind, TurnoverLayer, ValueKind};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_mp_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub layer: Option<String>,
    pub turnover_code: Option<String>,
    pub registrator_ref: Option<String>,
    pub general_ledger_ref: Option<String>,
}

pub async fn list(
    Query(params): Query<ListParams>,
) -> Result<Json<WbAdvertByItemListResponse>, StatusCode> {
    let limit = params.limit.or(Some(1000));
    let offset = params.offset.or(Some(0));
    let sort_desc = params.sort_desc.unwrap_or(true);

    let total_count = crate::projections::p911_wb_advert_by_items::service::count_with_filters(
        params.date_from.clone(),
        params.date_to.clone(),
        params.connection_mp_ref.clone(),
        params.nomenclature_ref.clone(),
        params.layer.clone(),
        params.turnover_code.clone(),
        params.registrator_ref.clone(),
        params.general_ledger_ref.clone(),
    )
    .await
    .map_err(|error| {
        tracing::error!("Failed to count p911 rows: {}", error);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items = crate::projections::p911_wb_advert_by_items::service::list_with_filters(
        params.date_from,
        params.date_to,
        params.connection_mp_ref,
        params.nomenclature_ref,
        params.layer,
        params.turnover_code,
        params.registrator_ref,
        params.general_ledger_ref,
        params.sort_by,
        sort_desc,
        offset,
        limit,
    )
    .await
    .map_err(|error| {
        tracing::error!("Failed to list p911 rows: {}", error);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dtos = items.into_iter().map(model_to_dto).collect::<Vec<_>>();
    let limit_value = limit.unwrap_or(1000);
    Ok(Json(WbAdvertByItemListResponse {
        total_count: total_count as i32,
        has_more: (offset.unwrap_or(0) + dtos.len() as u64) < total_count
            && (dtos.len() as u64) >= limit_value,
        items: dtos,
    }))
}

pub async fn get_by_general_ledger_ref(
    Path(general_ledger_ref): Path<String>,
) -> Result<Json<WbAdvertByItemDetailDto>, StatusCode> {
    let items = crate::projections::p911_wb_advert_by_items::service::list_by_general_ledger_ref(
        &general_ledger_ref,
    )
    .await
    .map_err(|error| {
        tracing::error!(
            "Failed to load p911 detail '{}': {}",
            general_ledger_ref,
            error
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if items.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let general_ledger_entry = crate::general_ledger::repository::get_by_id(&general_ledger_ref)
        .await
        .map_err(|error| {
            tracing::error!(
                "Failed to load general_ledger entry '{}' for p911 detail: {}",
                general_ledger_ref,
                error
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(to_general_ledger_dto);

    let total_amount = items.iter().map(|item| item.amount).sum();

    Ok(Json(WbAdvertByItemDetailDto {
        general_ledger_ref,
        general_ledger_entry,
        items: items.into_iter().map(model_to_dto).collect(),
        total_amount,
    }))
}

pub(crate) fn model_to_dto(
    model: crate::projections::p911_wb_advert_by_items::repository::Model,
) -> WbAdvertByItemDto {
    let class =
        crate::shared::analytics::turnover_registry::get_turnover_class(&model.turnover_code)
            .unwrap_or_else(|| panic!("Missing turnover class for {}", model.turnover_code));

    WbAdvertByItemDto {
        id: model.id,
        connection_mp_ref: model.connection_mp_ref,
        entry_date: model.entry_date,
        layer: TurnoverLayer::from_str(&model.layer).unwrap_or(TurnoverLayer::Oper),
        turnover_code: model.turnover_code,
        value_kind: ValueKind::from_str(&model.value_kind).unwrap_or(ValueKind::Money),
        agg_kind: AggKind::from_str(&model.agg_kind).unwrap_or(AggKind::Sum),
        amount: model.amount,
        nomenclature_ref: model.nomenclature_ref,
        registrator_type: model.registrator_type,
        registrator_ref: model.registrator_ref,
        general_ledger_ref: model.general_ledger_ref,
        is_problem: model.is_problem,
        created_at: model.created_at,
        updated_at: model.updated_at,
        turnover_name: class.name.to_string(),
        turnover_description: class.description.to_string(),
        turnover_llm_description: class.llm_description.to_string(),
        selection_rule: class.selection_rule,
        report_group: class.report_group,
    }
}

fn to_general_ledger_dto(row: crate::general_ledger::repository::Model) -> GeneralLedgerEntryDto {
    let comment =
        crate::shared::analytics::turnover_registry::get_turnover_class(&row.turnover_code)
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
