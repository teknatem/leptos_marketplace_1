use axum::{extract::Path, Json};
use serde_json::json;

use crate::domain::a007_marketplace_product;

/// GET /api/marketplace_product
pub async fn list_all() -> Result<
    Json<Vec<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct>>,
    axum::http::StatusCode,
> {
    match a007_marketplace_product::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/marketplace_product/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<
    Json<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct>,
    axum::http::StatusCode,
> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a007_marketplace_product::service::get_by_id(uuid).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/marketplace_product
pub async fn upsert(
    Json(dto): Json<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let result = if dto.id.is_some() {
        a007_marketplace_product::service::update(dto)
            .await
            .map(|_| uuid::Uuid::nil().to_string())
    } else {
        a007_marketplace_product::service::create(dto)
            .await
            .map(|id| id.to_string())
    };
    match result {
        Ok(id) => Ok(Json(json!({"id": id}))),
        Err(e) => {
            tracing::error!("Failed to save marketplace_product: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/marketplace_product/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a007_marketplace_product::service::delete(uuid).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/marketplace_product/testdata
pub async fn insert_test_data() -> axum::http::StatusCode {
    match a007_marketplace_product::service::insert_test_data().await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(e) => {
            tracing::error!("Failed to insert test data: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
