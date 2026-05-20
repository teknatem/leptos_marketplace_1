use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use contracts::system::favorites::{
    FavoriteDto, FavoriteTargetRequest, FavoriteUpdateRequest, FavoriteUpsertRequest,
};

use crate::system::auth::extractor::CurrentUser;
use crate::system::favorites::service;

fn map_error(err: anyhow::Error) -> StatusCode {
    let message = err.to_string();
    if message.contains("Forbidden") {
        StatusCode::FORBIDDEN
    } else if message.contains("not found") {
        StatusCode::NOT_FOUND
    } else if message.contains("Unsupported") || message.contains("required") {
        StatusCode::BAD_REQUEST
    } else {
        tracing::error!("Favorites API error: {}", message);
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub async fn list(CurrentUser(claims): CurrentUser) -> Result<Json<Vec<FavoriteDto>>, StatusCode> {
    service::list_visible(&claims.sub)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn get_target(
    CurrentUser(claims): CurrentUser,
    Query(query): Query<FavoriteTargetRequest>,
) -> Result<Json<Option<FavoriteDto>>, StatusCode> {
    service::get_personal_target(&claims.sub, &query.target_kind, &query.target_id)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn upsert(
    CurrentUser(claims): CurrentUser,
    Json(req): Json<FavoriteUpsertRequest>,
) -> Result<Json<FavoriteDto>, StatusCode> {
    service::upsert(&claims.sub, req)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn update(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
    Json(req): Json<FavoriteUpdateRequest>,
) -> Result<Json<FavoriteDto>, StatusCode> {
    service::update(&claims.sub, claims.is_admin, &id, req)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn delete(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    service::delete(&claims.sub, claims.is_admin, &id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(map_error)
}
