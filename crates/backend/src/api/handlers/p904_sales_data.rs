use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;

use crate::projections::p904_sales_data::repository::ModelWithCabinet;
use crate::projections::p904_sales_data::service;

#[derive(Deserialize)]
pub struct ListParams {
    pub limit: Option<u64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_mp_ref: Option<String>,
}

pub async fn list(
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<ModelWithCabinet>>, StatusCode> {
    // Валидация лимита: минимум 100, максимум 100000, по умолчанию 1000
    let limit = match params.limit {
        Some(lim) if lim < 100 => {
            tracing::warn!(
                "P904: Invalid limit {} (too small), using default 1000",
                lim
            );
            Some(1000)
        }
        Some(lim) if lim > 100000 => {
            tracing::warn!("P904: Invalid limit {} (too large), using max 100000", lim);
            Some(100000)
        }
        Some(lim) => Some(lim),
        None => {
            tracing::info!("P904: No limit specified, using default 1000");
            Some(1000)
        }
    };

    tracing::info!(
        "P904 list request: limit={:?}, date_from={:?}, date_to={:?}, connection_mp_ref={:?}",
        limit,
        params.date_from,
        params.date_to,
        params.connection_mp_ref
    );

    match service::list_with_filters(
        params.date_from,
        params.date_to,
        params.connection_mp_ref,
        limit,
    )
    .await
    {
        Ok(items) => {
            tracing::info!("P904 list response: {} items returned", items.len());
            Ok(Json(items))
        }
        Err(e) => {
            tracing::error!("Failed to list sales data: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
