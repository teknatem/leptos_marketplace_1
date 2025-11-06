use axum::Json;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use uuid::Uuid;

use crate::domain::a012_wb_sales;
use crate::shared::data::raw_storage;

/// Handler для получения списка Wildberries Sales
pub async fn list_sales() -> Result<Json<Vec<WbSales>>, axum::http::StatusCode> {
    let items = a012_wb_sales::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to list Wildberries sales: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(items))
}

/// Handler для получения детальной информации о Wildberries Sale
pub async fn get_sale_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<WbSales>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a012_wb_sales::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Wildberries sale detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

/// Handler для получения raw JSON от WB API по raw_payload_ref
pub async fn get_raw_json(
    axum::extract::Path(ref_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let raw_json_str = raw_storage::get_by_ref(&ref_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get raw JSON: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let json_value: serde_json::Value = serde_json::from_str(&raw_json_str)
        .map_err(|e| {
            tracing::error!("Failed to parse raw JSON: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json_value))
}

