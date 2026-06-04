use axum::{extract::{Path, Query}, Json};
use contracts::domain::a034_ym_realization::aggregate::{YmRealization, YmRealizationLine};
use contracts::domain::common::AggregateId;
use contracts::general_ledger::GeneralLedgerEntryDto;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::a034_ym_realization;
use crate::domain::a034_ym_realization::repository::{YmRealizationListQuery, YmRealizationListRow};

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
pub struct YmRealizationListItemDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub lines_count: i32,
    pub total_sales_revenue: f64,
    pub total_return_revenue: f64,
    pub net_revenue: f64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
    pub is_posted: bool,
}

impl From<YmRealizationListRow> for YmRealizationListItemDto {
    fn from(row: YmRealizationListRow) -> Self {
        Self {
            id: row.id,
            document_no: row.document_no,
            document_date: row.document_date,
            lines_count: row.lines_count,
            total_sales_revenue: row.total_sales_revenue,
            total_return_revenue: row.total_return_revenue,
            net_revenue: row.net_revenue,
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
    pub items: Vec<YmRealizationListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct YmRealizationDetailsDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_id: String,
    pub organization_name: Option<String>,
    pub marketplace_id: String,
    pub total_sales_revenue: f64,
    pub total_return_revenue: f64,
    pub net_revenue: f64,
    pub source: String,
    pub fetched_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_posted: bool,
    pub lines: Vec<YmRealizationLine>,
}

pub async fn list_paginated(
    Query(query): Query<ListQuery>,
) -> Result<Json<PaginatedResponse>, axum::http::StatusCode> {
    let page_size = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let page = if page_size > 0 { offset / page_size } else { 0 };

    let list_query = YmRealizationListQuery {
        date_from: query.date_from,
        date_to: query.date_to,
        connection_id: query.connection_id,
        search_query: query.search_query,
        sort_by: query.sort_by.unwrap_or_else(|| "document_date".to_string()),
        sort_desc: query.sort_desc.unwrap_or(true),
        limit: page_size,
        offset,
    };

    match a034_ym_realization::service::list_paginated(list_query).await {
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
            tracing::error!("Failed to list YM realization documents: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<YmRealizationDetailsDto>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let doc = match a034_ym_realization::service::get_by_id(uuid).await {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(axum::http::StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get YM realization document {}: {}", id, e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match build_details_dto(doc).await {
        Ok(dto) => Ok(Json(dto)),
        Err(e) => {
            tracing::error!("Failed to enrich YM realization document {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn build_details_dto(doc: YmRealization) -> anyhow::Result<YmRealizationDetailsDto> {
    let connection_name = resolve_connection_name(&doc.header.connection_id).await?;
    let organization_name = resolve_organization_name(&doc.header.organization_id).await?;

    Ok(YmRealizationDetailsDto {
        id: doc.base.id.as_string(),
        document_no: doc.header.document_no.clone(),
        document_date: doc.header.document_date.clone(),
        connection_id: doc.header.connection_id.clone(),
        connection_name,
        organization_id: doc.header.organization_id.clone(),
        organization_name,
        marketplace_id: doc.header.marketplace_id.clone(),
        total_sales_revenue: doc.totals.sales_revenue,
        total_return_revenue: doc.totals.return_revenue,
        net_revenue: doc.totals.net_revenue,
        source: doc.source_meta.source.clone(),
        fetched_at: doc.source_meta.fetched_at.clone(),
        created_at: doc.base.metadata.created_at.to_rfc3339(),
        updated_at: doc.base.metadata.updated_at.to_rfc3339(),
        is_posted: doc.is_posted || doc.base.metadata.is_posted,
        lines: doc.lines,
    })
}

async fn resolve_connection_name(connection_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = Uuid::parse_str(connection_id).ok() else {
        return Ok(None);
    };
    let connection = crate::domain::a006_connection_mp::service::get_by_id(uuid).await?;
    Ok(connection.map(|item| item.base.description))
}

async fn resolve_organization_name(organization_id: &str) -> anyhow::Result<Option<String>> {
    let Some(uuid) = Uuid::parse_str(organization_id).ok() else {
        return Ok(None);
    };
    let organization = crate::domain::a002_organization::service::get_by_id(uuid).await?;
    Ok(organization.map(|item| item.base.description))
}

pub async fn post_document(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    a034_ym_realization::service::post_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to post YM realization document {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({"success": true})))
}

pub async fn unpost_document(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    a034_ym_realization::service::unpost_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unpost YM realization document {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({"success": true})))
}

pub async fn get_general_ledger_entries(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let rows = crate::general_ledger::repository::list_by_registrator("a034_ym_realization", &id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get general ledger entries for a034 {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let general_ledger_entries = rows
        .into_iter()
        .map(to_journal_dto)
        .collect::<Vec<_>>();

    Ok(Json(
        serde_json::json!({ "general_ledger_entries": general_ledger_entries }),
    ))
}

fn to_journal_dto(row: crate::general_ledger::repository::Model) -> GeneralLedgerEntryDto {
    crate::general_ledger::dto::entry_to_dto(row)
}
