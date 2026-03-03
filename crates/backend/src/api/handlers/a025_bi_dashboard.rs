use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::a025_bi_dashboard;
use contracts::domain::a025_bi_dashboard::aggregate::BiDashboard;

#[derive(Deserialize)]
pub struct BiDashboardListParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Serialize)]
pub struct BiDashboardPaginatedResponse {
    pub items: Vec<BiDashboard>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// GET /api/a025-bi-dashboard
pub async fn list_all() -> Result<Json<Vec<BiDashboard>>, axum::http::StatusCode> {
    match a025_bi_dashboard::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a025-bi-dashboard/list
pub async fn list_paginated(
    Query(params): Query<BiDashboardListParams>,
) -> Result<Json<BiDashboardPaginatedResponse>, axum::http::StatusCode> {
    let limit = params.limit.unwrap_or(100).clamp(10, 10000);
    let offset = params.offset.unwrap_or(0);
    let page = offset / limit;

    match a025_bi_dashboard::service::list_paginated(page, limit).await {
        Ok((items, total)) => {
            let page_size = limit as usize;
            let page_num = (offset as usize) / page_size;
            let total_pages = ((total as usize) + page_size - 1) / page_size;

            Ok(Json(BiDashboardPaginatedResponse {
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

/// GET /api/a025-bi-dashboard/owner/:user_id
pub async fn list_by_owner(
    Path(user_id): Path<String>,
) -> Result<Json<Vec<BiDashboard>>, axum::http::StatusCode> {
    match a025_bi_dashboard::service::list_by_owner(&user_id).await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a025-bi-dashboard/public
pub async fn list_public() -> Result<Json<Vec<BiDashboard>>, axum::http::StatusCode> {
    match a025_bi_dashboard::service::list_public().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a025-bi-dashboard/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<BiDashboard>, axum::http::StatusCode> {
    match a025_bi_dashboard::service::get_by_id(&id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /api/a025-bi-dashboard/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    match a025_bi_dashboard::service::delete(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a025-bi-dashboard (upsert)
pub async fn upsert(
    Json(dto): Json<a025_bi_dashboard::service::BiDashboardDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    if dto.id.is_some() {
        match a025_bi_dashboard::service::update(dto).await {
            Ok(_) => Ok(Json(json!({"success": true}))),
            Err(e) => {
                tracing::error!("Failed to update BI dashboard: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        match a025_bi_dashboard::service::create(dto).await {
            Ok(id) => Ok(Json(json!({"success": true, "id": id.to_string()}))),
            Err(e) => {
                tracing::error!("Failed to create BI dashboard: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// POST /api/a025-bi-dashboard/testdata
pub async fn insert_test_data() -> axum::http::StatusCode {
    match a025_bi_dashboard::service::insert_test_data().await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(e) => {
            tracing::error!("Failed to insert BI dashboard test data: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
