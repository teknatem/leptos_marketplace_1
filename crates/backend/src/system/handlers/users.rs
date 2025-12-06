use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use contracts::system::users::{ChangePasswordDto, CreateUserDto, UpdateUserDto, User};

use crate::system::auth::extractor::CurrentUser;
use crate::system::users::service;

/// List all users (admin only)
pub async fn list(CurrentUser(_claims): CurrentUser) -> Result<Json<Vec<User>>, StatusCode> {
    let users = service::list_all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(users))
}

/// Get user by ID (admin only)
pub async fn get_by_id(
    CurrentUser(_claims): CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<User>, StatusCode> {
    let user = service::get_by_id(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(user))
}

/// Create user (admin only)
pub async fn create(
    CurrentUser(claims): CurrentUser,
    Json(dto): Json<CreateUserDto>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = service::create(dto, Some(claims.sub)).await.map_err(|e| {
        tracing::error!("Failed to create user: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    Ok(Json(serde_json::json!({"id": user_id})))
}

/// Update user (admin only)
pub async fn update(
    CurrentUser(_claims): CurrentUser,
    Path(id): Path<String>,
    Json(mut dto): Json<UpdateUserDto>,
) -> Result<StatusCode, StatusCode> {
    // Ensure ID matches
    dto.id = id;

    service::update(dto).await.map_err(|e| {
        tracing::error!("Failed to update user: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    Ok(StatusCode::OK)
}

/// Delete user (admin only)
pub async fn delete(
    CurrentUser(_claims): CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let deleted = service::delete(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Change password
pub async fn change_password(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
    Json(mut dto): Json<ChangePasswordDto>,
) -> Result<StatusCode, StatusCode> {
    // Ensure user_id matches
    dto.user_id = id;

    service::change_password(dto, &claims.sub)
        .await
        .map_err(|e| {
            tracing::error!("Failed to change password: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    Ok(StatusCode::OK)
}
