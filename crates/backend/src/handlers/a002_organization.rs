use axum::{extract::Path, Json};
use serde_json::json;

use crate::domain::a002_organization;

/// GET /api/organization
pub async fn list_all() -> Result<
    Json<Vec<contracts::domain::a002_organization::aggregate::Organization>>,
    axum::http::StatusCode,
> {
    match a002_organization::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/organization/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<
    Json<contracts::domain::a002_organization::aggregate::Organization>,
    axum::http::StatusCode,
> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a002_organization::service::get_by_id(uuid).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/organization
pub async fn upsert(
    Json(dto): Json<contracts::domain::a002_organization::aggregate::OrganizationDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let result = if dto.id.is_some() {
        a002_organization::service::update(dto)
            .await
            .map(|_| uuid::Uuid::nil().to_string())
    } else {
        a002_organization::service::create(dto)
            .await
            .map(|id| id.to_string())
    };

    match result {
        Ok(id) => Ok(Json(json!({"id": id}))),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /api/organization/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a002_organization::service::delete(uuid).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/organization/testdata
pub async fn insert_test_data() -> axum::http::StatusCode {
    match a002_organization::service::insert_test_data().await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}
