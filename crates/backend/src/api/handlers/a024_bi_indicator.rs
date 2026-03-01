use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::a024_bi_indicator;
use contracts::domain::a024_bi_indicator::aggregate::BiIndicator;

#[derive(Deserialize)]
pub struct BiIndicatorListParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Serialize)]
pub struct BiIndicatorPaginatedResponse {
    pub items: Vec<BiIndicator>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// GET /api/a024-bi-indicator
pub async fn list_all() -> Result<Json<Vec<BiIndicator>>, axum::http::StatusCode> {
    match a024_bi_indicator::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a024-bi-indicator/list
pub async fn list_paginated(
    Query(params): Query<BiIndicatorListParams>,
) -> Result<Json<BiIndicatorPaginatedResponse>, axum::http::StatusCode> {
    let limit = params.limit.unwrap_or(100).clamp(10, 10000);
    let offset = params.offset.unwrap_or(0);
    let page = offset / limit;

    match a024_bi_indicator::service::list_paginated(page, limit).await {
        Ok((items, total)) => {
            let page_size = limit as usize;
            let page_num = (offset as usize) / page_size;
            let total_pages = ((total as usize) + page_size - 1) / page_size;

            Ok(Json(BiIndicatorPaginatedResponse {
                items,
                total,
                page: page_num,
                page_size,
                total_pages,
            }))
        }
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a024-bi-indicator/owner/:user_id
pub async fn list_by_owner(
    Path(user_id): Path<String>,
) -> Result<Json<Vec<BiIndicator>>, axum::http::StatusCode> {
    match a024_bi_indicator::service::list_by_owner(&user_id).await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a024-bi-indicator/public
pub async fn list_public() -> Result<Json<Vec<BiIndicator>>, axum::http::StatusCode> {
    match a024_bi_indicator::service::list_public().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a024-bi-indicator/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<BiIndicator>, axum::http::StatusCode> {
    match a024_bi_indicator::service::get_by_id(&id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /api/a024-bi-indicator/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    match a024_bi_indicator::service::delete(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a024-bi-indicator
pub async fn upsert(
    Json(dto): Json<a024_bi_indicator::service::BiIndicatorDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    if dto.id.is_some() {
        match a024_bi_indicator::service::update(dto).await {
            Ok(_) => Ok(Json(json!({"success": true}))),
            Err(e) => {
                tracing::error!("Failed to update BI indicator: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        match a024_bi_indicator::service::create(dto).await {
            Ok(id) => Ok(Json(json!({"success": true, "id": id.to_string()}))),
            Err(e) => {
                tracing::error!("Failed to create BI indicator: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
