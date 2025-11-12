use axum::Json;
use uuid::Uuid;

use crate::domain::a014_ozon_transactions;

/// Handler для получения списка всех транзакций
pub async fn list_all() -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let transactions = a014_ozon_transactions::service::list_all_as_dto()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list OZON transactions: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!(transactions)))
}

/// Handler для получения транзакции по ID
pub async fn get_by_id(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let transaction = a014_ozon_transactions::service::get_by_id_as_dto(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get OZON transaction by ID: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!(transaction)))
}

/// Handler для удаления транзакции
pub async fn delete(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let deleted = a014_ozon_transactions::service::delete(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete OZON transaction: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !deleted {
        return Err(axum::http::StatusCode::NOT_FOUND);
    }

    Ok(Json(serde_json::json!({"success": true})))
}
