use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use contracts::system::roles::{CreateRoleDto, Role, RoleScopeAccess, UpdateRoleDto};

use crate::system::roles::service;

pub async fn list_roles() -> Result<Json<Vec<Role>>, StatusCode> {
    service::list_all()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn create_role(
    Json(dto): Json<CreateRoleDto>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match service::create(dto).await {
        Ok(id) => Ok(Json(serde_json::json!({ "id": id }))),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already exists") || msg.contains("empty") {
                Err(StatusCode::BAD_REQUEST)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn update_role(
    Path(id): Path<String>,
    Json(mut dto): Json<UpdateRoleDto>,
) -> Result<StatusCode, StatusCode> {
    dto.id = id;
    match service::update(dto).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("system role") || msg.contains("not found") || msg.contains("empty") {
                Err(StatusCode::BAD_REQUEST)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn delete_role(Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    match service::delete(&id).await {
        Ok(true) => Ok(StatusCode::OK),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("system role") || msg.contains("not found") {
                Err(StatusCode::BAD_REQUEST)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn get_role_permissions(
    Path(id): Path<String>,
) -> Result<Json<Vec<RoleScopeAccess>>, StatusCode> {
    service::get_permissions(&id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// PUT /api/system/roles/:id/permissions
/// Replace all scope grants for a custom role.
pub async fn update_role_permissions(
    Path(id): Path<String>,
    Json(grants): Json<Vec<RoleScopeAccess>>,
) -> Result<StatusCode, StatusCode> {
    match service::update_permissions(&id, grants).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("system role") || msg.contains("not found") || msg.contains("Invalid") {
                Err(StatusCode::BAD_REQUEST)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
