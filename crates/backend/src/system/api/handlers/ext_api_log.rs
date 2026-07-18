//! Хендлеры статистики по входящему внешнему API (`/api/ext/v1/*`).
//!
//! Только чтение: пишет данные слой `system::ext_api_log::middleware`.

use axum::{extract::Query, Json};
use contracts::system::ext_api_log::{
    ExtApiHistoryRequest, ExtApiHistoryResponse, ExtApiLogListResponse, ExtApiMetric, ExtApiScale,
    ExtApiSummaryResponse,
};
use serde::Deserialize;

use crate::system::ext_api_log::service;

/// GET /api/sys/ext-api/history — временной ряд для графика.
#[derive(Deserialize)]
pub struct ExtApiHistoryQuery {
    pub scale: ExtApiScale,
    pub metric: ExtApiMetric,
    pub date_from: String,
}

pub async fn ext_api_history(
    Query(q): Query<ExtApiHistoryQuery>,
) -> Result<Json<ExtApiHistoryResponse>, axum::http::StatusCode> {
    let req = ExtApiHistoryRequest {
        scale: q.scale,
        metric: q.metric,
        date_from: q.date_from,
    };
    match service::query_history(req).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => {
            tracing::error!("Failed to query ext api history: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sys/ext-api/summary — сводка за период по эндпоинтам и потребителям.
#[derive(Deserialize)]
pub struct ExtApiSummaryQuery {
    pub scale: ExtApiScale,
    pub date_from: String,
}

pub async fn ext_api_summary(
    Query(q): Query<ExtApiSummaryQuery>,
) -> Result<Json<ExtApiSummaryResponse>, axum::http::StatusCode> {
    match service::summary(&q.date_from, q.scale).await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => {
            tracing::error!("Failed to query ext api summary: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/sys/ext-api/recent — последние вызовы.
#[derive(Deserialize)]
pub struct ExtApiRecentQuery {
    pub limit: Option<u64>,
}

pub async fn ext_api_recent(
    Query(q): Query<ExtApiRecentQuery>,
) -> Result<Json<ExtApiLogListResponse>, axum::http::StatusCode> {
    let limit = q.limit.unwrap_or(100).min(1000);
    match service::list_recent(limit).await {
        Ok(rows) => Ok(Json(ExtApiLogListResponse { rows })),
        Err(e) => {
            tracing::error!("Failed to list recent ext api calls: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
