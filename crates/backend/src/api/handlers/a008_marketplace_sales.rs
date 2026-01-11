use axum::{extract::Path, Json};
use serde_json::json;

use crate::domain::a008_marketplace_sales;

/// GET /api/marketplace_sales
pub async fn list_all() -> Result<
    Json<Vec<contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSales>>,
    axum::http::StatusCode,
> {
    match a008_marketplace_sales::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/marketplace_sales/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<
    Json<contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSales>,
    axum::http::StatusCode,
> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a008_marketplace_sales::service::get_by_id(uuid).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/marketplace_sales
pub async fn upsert(
    Json(dto): Json<contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSalesDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let result = if dto.id.is_some() {
        a008_marketplace_sales::service::update(dto)
            .await
            .map(|_| uuid::Uuid::nil().to_string())
    } else {
        a008_marketplace_sales::service::create(dto)
            .await
            .map(|id| id.to_string())
    };
    match result {
        Ok(id) => Ok(Json(json!({"id": id}))),
        Err(e) => {
            tracing::error!("Failed to save marketplace_sales: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/marketplace_sales/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a008_marketplace_sales::service::delete(uuid).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}
