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
    match service::list_with_filters(
        params.date_from,
        params.date_to,
        params.connection_mp_ref,
        params.limit,
    )
    .await
    {
        Ok(items) => Ok(Json(items)),
        Err(e) => {
            tracing::error!("Failed to list sales data: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

