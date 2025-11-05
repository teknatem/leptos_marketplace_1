use axum::Json;
use contracts::domain::a011_ozon_fbo_posting::aggregate::OzonFboPosting;
use uuid::Uuid;

use crate::domain::a011_ozon_fbo_posting;

/// Handler для получения списка OZON FBO Posting
pub async fn list_postings() -> Result<Json<Vec<OzonFboPosting>>, axum::http::StatusCode> {
    let items = a011_ozon_fbo_posting::service::list_all()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list OZON FBO postings: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(items))
}

/// Handler для получения детальной информации о OZON FBO Posting
pub async fn get_posting_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<OzonFboPosting>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a011_ozon_fbo_posting::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get OZON FBO posting detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

