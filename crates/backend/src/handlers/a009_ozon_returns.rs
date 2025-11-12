use axum::Json;
use uuid::Uuid;

use crate::domain::a009_ozon_returns;

/// Handler для проведения документа возврата
pub async fn post_ozon_return(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a009_ozon_returns::posting::post_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to post OZON return: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// Handler для отмены проведения документа возврата
pub async fn unpost_ozon_return(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    a009_ozon_returns::posting::unpost_document(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unpost OZON return: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({"success": true})))
}
