use axum::{
    extract::{Path, Query},
    Json,
};
use contracts::domain::a026_wb_advert_daily::aggregate::{WbAdvertDaily, WbAdvertDailyMetrics};
use contracts::domain::common::AggregateId;
use contracts::projections::general_ledger::GeneralLedgerEntryDto;
use contracts::shared::analytics::TurnoverLayer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::a026_wb_advert_daily;
use crate::domain::a026_wb_advert_daily::repository::{
    WbAdvertDailyListQuery, WbAdvertDailyListRow,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search_query: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct WbAdvertDailyListItemDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub lines_count: i32,
    pub total_views: i64,
    pub total_clicks: i64,
    pub total_orders: i64,
    pub total_sum: f64,
    pub total_sum_price: f64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
    pub is_posted: bool,
}

impl From<WbAdvertDailyListRow> for WbAdvertDailyListItemDto {
    fn from(row: WbAdvertDailyListRow) -> Self {
        Self {
            id: row.id,
            document_no: row.document_no,
            document_date: row.document_date,
            lines_count: row.lines_count,
            total_views: row.total_views,
            total_clicks: row.total_clicks,
            total_orders: row.total_orders,
            total_sum: row.total_sum,
            total_sum_price: row.total_sum_price,
            connection_id: row.connection_id,
            connection_name: row.connection_name,
            organization_name: row.organization_name,
            fetched_at: row.fetched_at,
            is_posted: row.is_posted,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse {
    pub items: Vec<WbAdvertDailyListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WbAdvertDailyLineDetailsDto {
    pub nm_id: i64,
    pub wb_name: String,
    pub nomenclature_ref: Option<String>,
    pub nomenclature_article: Option<String>,
    pub nomenclature_name: Option<String>,
    pub advert_ids: Vec<i64>,
    pub app_types: Vec<i32>,
    pub metrics: WbAdvertDailyMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct WbAdvertDailyDetailsDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_id: String,
    pub organization_name: Option<String>,
    pub marketplace_id: String,
    pub marketplace_name: Option<String>,
    pub totals: WbAdvertDailyMetrics,
    pub unattributed_totals: WbAdvertDailyMetrics,
    pub source: String,
    pub fetched_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_posted: bool,
    pub lines: Vec<WbAdvertDailyLineDetailsDto>,
}

pub async fn list_paginated(
    Query(query): Query<ListQuery>,
) -> Result<Json<PaginatedResponse>, axum::http::StatusCode> {
    let page_size = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let page = if page_size > 0 { offset / page_size } else { 0 };

    let list_query = WbAdvertDailyListQuery {
        date_from: query.date_from,
        date_to: query.date_to,
        connection_id: query.connection_id,
        search_query: query.search_query,
        sort_by: query.sort_by.unwrap_or_else(|| "document_date".to_string()),
        sort_desc: query.sort_desc.unwrap_or(true),
        limit: page_size,
        offset,
    };

    match a026_wb_advert_daily::service::list_paginated(list_query).await {
        Ok(result) => {
            let total_pages = if page_size > 0 {
                (result.total + page_size - 1) / page_size
            } else {
                1
            };
            Ok(Json(PaginatedResponse {
                items: result.items.into_iter().map(Into::into).collect(),
                total: result.total,
                page,
                page_size,
                total_pages,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list WB advert daily documents: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<WbAdvertDailyDetailsDto>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let doc = match a026_wb_advert_daily::service::get_by_id(uuid).await {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(axum::http::StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get WB advert daily document {}: {}", id, e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match build_details_dto(doc).await {
        Ok(dto) => Ok(Json(dto)),
        Err(e) => {
            tracing::error!("Failed to enrich WB advert daily document {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn build_details_dto(doc: WbAdvertDaily) -> anyhow::Result<WbAdvertDailyDetailsDto> {
    let connection_name = resolve_connection_name(&doc.header.connection_id).await?;
    let organization_name = resolve_organization_name(&doc.header.organization_id).await?;
    let marketplace_name = resolve_marketplace_name(&doc.header.marketplace_id).await?;

    let mut nomenclature_cache: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    for line in &doc.lines {
        let Some(nom_ref) = line.nomenclature_ref.as_ref() else {
            continue;
        };
        if nomenclature_cache.contains_key(nom_ref) {
            continue;
        }

        let Some(uuid) = parse_uuid(nom_ref) else {
            nomenclature_cache.insert(nom_ref.clone(), (None, None));
            continue;
        };

        let nomenclature = crate::domain::a004_nomenclature::service::get_by_id(uuid).await?;
        let cached = nomenclature.map_or((None, None), |nom| {
            (Some(nom.article), Some(nom.base.description))
        });
        nomenclature_cache.insert(nom_ref.clone(), cached);
    }

    let lines = doc
        .lines
        .iter()
        .map(|line| {
            let (article, name) = line
                .nomenclature_ref
                .as_ref()
                .and_then(|nom_ref| nomenclature_cache.get(nom_ref).cloned())
                .unwrap_or((None, None));

            WbAdvertDailyLineDetailsDto {
                nm_id: line.nm_id,
                wb_name: line.nm_name.clone(),
                nomenclature_ref: line.nomenclature_ref.clone(),
                nomenclature_article: article,
                nomenclature_name: name,
                advert_ids: line.advert_ids.clone(),
                app_types: line.app_types.clone(),
                metrics: line.metrics.clone(),
            }
        })
        .collect();

    Ok(WbAdvertDailyDetailsDto {
        id: doc.base.id.as_string(),
        document_no: doc.header.document_no.clone(),
        document_date: doc.header.document_date.clone(),
        connection_id: doc.header.connection_id.clone(),
        connection_name,
        organization_id: doc.header.organization_id.clone(),
        organization_name,
        marketplace_id: doc.header.marketplace_id.clone(),
        marketplace_name,
        totals: doc.totals.clone(),
        unattributed_totals: doc.unattributed_totals.clone(),
        source: doc.source_meta.source.clone(),
        fetched_at: doc.source_meta.fetched_at.clone(),
        created_at: doc.base.metadata.created_at.to_rfc3339(),
        updated_at: doc.base.metadata.updated_at.to_rfc3339(),
        is_posted: doc.is_posted || doc.base.metadata.is_posted,
        lines,
    })
}

async fn resolve_connection_name(connection_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = parse_uuid(connection_id) else {
        return Ok(None);
    };
    let connection = crate::domain::a006_connection_mp::service::get_by_id(uuid).await?;
    Ok(connection.map(|item| item.base.description))
}

async fn resolve_organization_name(organization_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = parse_uuid(organization_id) else {
        return Ok(None);
    };
    let organization = crate::domain::a002_organization::service::get_by_id(uuid).await?;
    Ok(organization.map(|item| item.base.description))
}

async fn resolve_marketplace_name(marketplace_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = parse_uuid(marketplace_id) else {
        return Ok(None);
    };
    let marketplace = crate::domain::a005_marketplace::service::get_by_id(uuid).await?;
    Ok(marketplace.map(|item| item.base.description))
}

fn parse_uuid(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

pub async fn post_document(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a026_wb_advert_daily::posting::post_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to post WB advert daily document {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

pub async fn unpost_document(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a026_wb_advert_daily::posting::unpost_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unpost WB advert daily document {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

pub async fn get_projections(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let registrator_ref = format!("a026:{}", id);
    let p911_items = crate::projections::p911_wb_advert_by_items::service::list_by_registrator_ref(
        &registrator_ref,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get p911 projections for {}: {}", id, e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "p911_wb_advert_by_items": p911_items
    })))
}

pub async fn get_general_ledger_entries(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let registrator_ref = format!("a026:{}", id);
    let rows = crate::projections::general_ledger::repository::list_with_filters(
        None,
        None,
        Some(registrator_ref),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to get general ledger entries for a026 {}: {}",
            id,
            e
        );
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let general_ledger_entries = rows.into_iter().map(to_journal_dto).collect::<Vec<_>>();

    Ok(Json(
        serde_json::json!({ "general_ledger_entries": general_ledger_entries }),
    ))
}

fn to_journal_dto(
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
