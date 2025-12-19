use axum::Json;

use crate::shared::logger;

/// GET /api/logs
pub async fn list_all(
) -> Result<Json<Vec<contracts::shared::logger::LogEntry>>, axum::http::StatusCode> {
    match logger::repository::get_all_logs().await {
        Ok(logs) => Ok(Json(logs)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/logs
pub async fn create(
    Json(req): Json<contracts::shared::logger::CreateLogRequest>,
) -> axum::http::StatusCode {
    match logger::repository::log_event(&req.source, &req.category, &req.message).await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// DELETE /api/logs
pub async fn clear_all() -> axum::http::StatusCode {
    match logger::repository::clear_all_logs().await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}
