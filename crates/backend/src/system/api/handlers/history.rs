use axum::{extract::Query, http::StatusCode, Json};
use contracts::system::history::{PageHistoryDto, PageHistoryListQuery, PageHistoryRecordRequest};

use crate::system::auth::extractor::CurrentUser;
use crate::system::history::service;

fn map_error(err: anyhow::Error) -> StatusCode {
    let message = err.to_string();
    if message.contains("required") {
        StatusCode::BAD_REQUEST
    } else {
        tracing::error!("Page history API error: {}", message);
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub async fn list(
    CurrentUser(claims): CurrentUser,
    Query(query): Query<PageHistoryListQuery>,
) -> Result<Json<Vec<PageHistoryDto>>, StatusCode> {
    service::list_recent(&claims.sub, query.limit)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn record(
    CurrentUser(claims): CurrentUser,
    Json(req): Json<PageHistoryRecordRequest>,
) -> Result<Json<PageHistoryDto>, StatusCode> {
    service::record(&claims.sub, req)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn clear(CurrentUser(claims): CurrentUser) -> Result<StatusCode, StatusCode> {
    service::clear(&claims.sub)
        .await
        .map(|_| StatusCode::OK)
        .map_err(map_error)
}
