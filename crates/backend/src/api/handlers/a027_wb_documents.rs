use axum::{
    body::Body,
    extract::{Path, Query},
    http::{
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
        HeaderValue, StatusCode,
    },
    response::Response,
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use contracts::domain::a027_wb_documents::aggregate::{WbDocument, WbWeeklyReportManualData};
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::a027_wb_documents;
use crate::usecases::u504_import_from_wildberries::wildberries_api_client::WildberriesApiClient;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub weekly_only: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub search_query: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct WbDocumentsListItemDto {
    pub id: String,
    pub service_name: String,
    pub name: String,
    pub category: String,
    pub creation_time: String,
    pub is_weekly_report: bool,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub viewed: bool,
    pub extensions: Vec<String>,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse {
    pub items: Vec<WbDocumentsListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Serialize)]
pub struct WbDocumentDetailsDto {
    pub id: String,
    pub service_name: String,
    pub name: String,
    pub category: String,
    pub creation_time: String,
    pub viewed: bool,
    pub extensions: Vec<String>,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_id: String,
    pub organization_name: Option<String>,
    pub marketplace_id: String,
    pub marketplace_name: Option<String>,
    pub is_weekly_report: bool,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub realized_goods_total: Option<f64>,
    pub wb_reward_with_vat: Option<f64>,
    pub seller_transfer_total: Option<f64>,
    pub fetched_at: String,
    pub locale: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateManualFieldsRequest {
    pub is_weekly_report: bool,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub realized_goods_total: Option<f64>,
    pub wb_reward_with_vat: Option<f64>,
    pub seller_transfer_total: Option<f64>,
}

pub async fn list_paginated(
    Query(query): Query<ListQuery>,
) -> Result<Json<PaginatedResponse>, StatusCode> {
    let page_size = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let page = if page_size > 0 { offset / page_size } else { 0 };

    let list_query = a027_wb_documents::service::WbDocumentsListQuery {
        date_from: query.date_from,
        date_to: query.date_to,
        connection_id: query.connection_id,
        weekly_only: query.weekly_only.unwrap_or(false),
        search_query: query.search_query,
        sort_by: query.sort_by.unwrap_or_else(|| "creation_time".to_string()),
        sort_desc: query.sort_desc.unwrap_or(true),
        limit: page_size,
        offset,
    };

    match a027_wb_documents::service::list_paginated(list_query).await {
        Ok(result) => {
            let total_pages = if page_size > 0 {
                (result.total + page_size - 1) / page_size
            } else {
                1
            };

            Ok(Json(PaginatedResponse {
                items: result
                    .items
                    .into_iter()
                    .map(|row| WbDocumentsListItemDto {
                        id: row.id,
                        service_name: row.service_name,
                        name: row.name,
                        category: row.category,
                        creation_time: row.creation_time,
                        is_weekly_report: row.is_weekly_report,
                        report_period_from: row.report_period_from,
                        report_period_to: row.report_period_to,
                        viewed: row.viewed,
                        extensions: row.extensions,
                        connection_id: row.connection_id,
                        connection_name: row.connection_name,
                        organization_name: row.organization_name,
                        fetched_at: row.fetched_at,
                    })
                    .collect(),
                total: result.total,
                page,
                page_size,
                total_pages,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list WB documents: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_by_id(Path(id): Path<String>) -> Result<Json<WbDocumentDetailsDto>, StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let doc = match a027_wb_documents::service::get_by_id(uuid).await {
        Ok(Some(doc)) => doc,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get WB document {}: {}", id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    build_details_dto(doc).await.map(Json).map_err(|e| {
        tracing::error!("Failed to build WB document dto {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

pub async fn download_document(
    Path((id, extension)): Path<(String, String)>,
) -> Result<Response<Body>, StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let document = a027_wb_documents::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load WB document {} for download: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !document
        .header
        .extensions
        .iter()
        .any(|item| item.eq_ignore_ascii_case(&extension))
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let connection_id = Uuid::parse_str(&document.header.connection_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load connection for WB document {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let api_client = WildberriesApiClient::new();
    let file = api_client
        .download_document(&connection, &document.header.service_name, &extension)
        .await
        .map_err(|e| {
            tracing::error!("Failed to download WB document {}: {}", id, e);
            StatusCode::BAD_GATEWAY
        })?;

    let bytes = general_purpose::STANDARD
        .decode(&file.document)
        .map_err(|e| {
            tracing::error!("Failed to decode WB document base64 {}: {}", id, e);
            StatusCode::BAD_GATEWAY
        })?;

    let mut response = Response::new(Body::from(bytes));
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(content_type_for_extension(&file.extension)),
    );
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"{}\"",
            sanitize_filename(&file.file_name)
        ))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok(response)
}

pub async fn update_manual_fields(
    Path(id): Path<String>,
    Json(req): Json<UpdateManualFieldsRequest>,
) -> Result<Json<WbDocumentDetailsDto>, StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let document = a027_wb_documents::service::update_manual_fields(
        uuid,
        req.is_weekly_report,
        normalize_optional_string(req.report_period_from),
        normalize_optional_string(req.report_period_to),
        WbWeeklyReportManualData {
            realized_goods_total: req.realized_goods_total,
            wb_reward_with_vat: req.wb_reward_with_vat,
            seller_transfer_total: req.seller_transfer_total,
        },
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to update WB document manual fields {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    build_details_dto(document).await.map(Json).map_err(|e| {
        tracing::error!("Failed to build updated WB document dto {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn build_details_dto(doc: WbDocument) -> anyhow::Result<WbDocumentDetailsDto> {
    Ok(WbDocumentDetailsDto {
        id: doc.base.id.as_string(),
        service_name: doc.header.service_name.clone(),
        name: doc.header.name.clone(),
        category: doc.header.category.clone(),
        creation_time: doc.header.creation_time.clone(),
        viewed: doc.header.viewed,
        extensions: doc.header.extensions.clone(),
        connection_id: doc.header.connection_id.clone(),
        connection_name: resolve_connection_name(&doc.header.connection_id).await?,
        organization_id: doc.header.organization_id.clone(),
        organization_name: resolve_organization_name(&doc.header.organization_id).await?,
        marketplace_id: doc.header.marketplace_id.clone(),
        marketplace_name: resolve_marketplace_name(&doc.header.marketplace_id).await?,
        is_weekly_report: doc.is_weekly_report,
        report_period_from: doc.report_period_from.clone(),
        report_period_to: doc.report_period_to.clone(),
        realized_goods_total: doc.weekly_report_data.realized_goods_total,
        wb_reward_with_vat: doc.weekly_report_data.wb_reward_with_vat,
        seller_transfer_total: doc.weekly_report_data.seller_transfer_total,
        fetched_at: doc.source_meta.fetched_at.clone(),
        locale: doc.source_meta.locale.clone(),
        created_at: doc.base.metadata.created_at.to_rfc3339(),
        updated_at: doc.base.metadata.updated_at.to_rfc3339(),
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

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn sanitize_filename(value: &str) -> String {
    value.replace(['\r', '\n', '"'], "_")
}

fn content_type_for_extension(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        "zip" => "application/zip",
        "pdf" => "application/pdf",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "csv" => "text/csv; charset=utf-8",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }
}
