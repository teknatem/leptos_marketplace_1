use axum::Json;
use contracts::domain::a013_ym_order::aggregate::YmOrder;
use uuid::Uuid;

use crate::domain::a013_ym_order;
use crate::shared::data::raw_storage;

/// Handler для получения списка Yandex Market Orders
pub async fn list_orders() -> Result<Json<Vec<YmOrder>>, axum::http::StatusCode> {
    let items = a013_ym_order::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to list Yandex Market orders: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(items))
}

/// Handler для получения детальной информации о Yandex Market Order
pub async fn get_order_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<YmOrder>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a013_ym_order::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Yandex Market order detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

/// Handler для получения raw JSON от Yandex Market API по raw_payload_ref
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

