use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;

use crate::projections::p906_nomenclature_prices::repository::PriceWithNomenclature;
use crate::projections::p906_nomenclature_prices::service;

#[derive(Deserialize)]
pub struct ListParams {
    pub period: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub q: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(serde::Serialize)]
pub struct ListResponse {
    pub items: Vec<PriceWithNomenclature>,
    pub total_count: i64,
}

/// GET /api/p906/nomenclature-prices
pub async fn list(
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse>, StatusCode> {
    // Валидация лимита: минимум 10, максимум 10000, по умолчанию 1000
    let limit = match params.limit {
        Some(lim) if lim < 10 => {
            tracing::warn!("P906: Invalid limit {} (too small), using default 1000", lim);
            Some(1000)
        }
        Some(lim) if lim > 10000 => {
            tracing::warn!("P906: Invalid limit {} (too large), using max 10000", lim);
            Some(10000)
        }
        Some(lim) => Some(lim),
        None => {
            tracing::info!("P906: No limit specified, using default 1000");
            Some(1000)
        }
    };

    tracing::info!(
        "P906 list request: period={:?}, nomenclature_ref={:?}, limit={:?}, offset={:?}",
        params.period,
        params.nomenclature_ref,
        limit,
        params.offset
    );

    match service::list_with_filters(
        params.period,
        params.nomenclature_ref,
        params.q,
        params.sort_by,
        params.sort_desc,
        limit,
        params.offset,
    )
    .await
    {
        Ok((items, total_count)) => {
            tracing::info!("P906 list response: {} items returned, total_count={}", items.len(), total_count);
            Ok(Json(ListResponse { items, total_count }))
        }
        Err(e) => {
            tracing::error!("Failed to list nomenclature prices: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/p906/periods
/// Возвращает список уникальных периодов для фильтра в UI
pub async fn get_periods() -> Result<Json<Vec<String>>, StatusCode> {
    match service::get_unique_periods().await {
        Ok(periods) => {
            tracing::info!("P906 periods response: {} unique periods", periods.len());
            Ok(Json(periods))
        }
        Err(e) => {
            tracing::error!("Failed to get unique periods: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

