use axum::Json;
use contracts::domain::a010_ozon_fbs_posting::aggregate::OzonFbsPosting;
use uuid::Uuid;

use crate::domain::a010_ozon_fbs_posting;
use crate::shared::data::raw_storage;

/// Handler для получения списка OZON FBS Posting
pub async fn list_postings() -> Result<Json<Vec<OzonFbsPosting>>, axum::http::StatusCode> {
    let items = a010_ozon_fbs_posting::service::list_all()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list OZON FBS postings: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(items))
}

/// Handler для получения детальной информации о OZON FBS Posting
pub async fn get_posting_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<OzonFbsPosting>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a010_ozon_fbs_posting::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get OZON FBS posting detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

/// Handler для получения raw JSON от OZON API по raw_payload_ref
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

