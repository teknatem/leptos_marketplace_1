use axum::Json;
use uuid::Uuid;
use serde::Deserialize;

use crate::domain::a014_ozon_transactions;

#[derive(Debug, Deserialize)]
pub struct ListFilters {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub transaction_type: Option<String>,
    pub operation_type_name: Option<String>,
    pub posting_number: Option<String>,
}

/// Handler для получения списка всех транзакций с фильтрами
pub async fn list_all(
    axum::extract::Query(filters): axum::extract::Query<ListFilters>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let transactions = a014_ozon_transactions::service::list_with_filters_as_dto(
        filters.date_from,
        filters.date_to,
        filters.transaction_type,
        filters.operation_type_name,
        filters.posting_number,
    )
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

/// Handler для получения транзакций по posting_number
pub async fn get_by_posting_number(
    axum::extract::Path(posting_number): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    // Декодируем URL-кодированный posting_number
    let decoded_posting_number = urlencoding::decode(&posting_number)
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    
    tracing::info!("Getting transactions for posting_number: {} (original: {})", decoded_posting_number, posting_number);
    
    let transactions = a014_ozon_transactions::service::get_by_posting_number_as_dto(&decoded_posting_number)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get OZON transactions by posting_number: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Found {} transactions for posting_number: {}", transactions.len(), decoded_posting_number);
    Ok(Json(serde_json::json!(transactions)))
}