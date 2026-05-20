use axum::{extract::Path, extract::Query, Json};
use contracts::domain::a033_wb_day_close::{
    ArchiveAndRecreateRequest, CompareRequest, CompareResponse, CreateActiveRequest,
    RepostProblematicRequest, RepostResult, WbDayClose, WbDayCloseListDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::a033_wb_day_close::{repository::ListQuery, service};

// ─────────────────────────────────────────────────────────────────────────────
// Query params
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery_ {
    pub connection_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub include_archived: Option<bool>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/a033/wb-day-close
// ─────────────────────────────────────────────────────────────────────────────

pub async fn list_paginated(
    Query(q): Query<ListQuery_>,
) -> Result<Json<Vec<WbDayCloseListDto>>, axum::http::StatusCode> {
    let query = ListQuery {
        connection_id: q.connection_id,
        date_from: q.date_from,
        date_to: q.date_to,
        include_archived: q.include_archived,
        limit: q.limit,
        offset: q.offset,
    };

    service::list_paginated(query)
        .await
        .map(|items| Json(items.into_iter().map(|d| d.to_list_dto()).collect()))
        .map_err(|e| {
            tracing::error!("list a033: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/a033/wb-day-close/:id
// ─────────────────────────────────────────────────────────────────────────────

pub async fn get_by_id(Path(id): Path<String>) -> Result<Json<WbDayClose>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    match service::get_by_id(uuid).await {
        Ok(Some(doc)) => Ok(Json(doc)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("get a033 {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/a033/wb-day-close/by-day/:connection_id/:business_date
// ─────────────────────────────────────────────────────────────────────────────

pub async fn list_by_day(
    Path((connection_id, business_date)): Path<(String, String)>,
) -> Result<Json<Vec<WbDayCloseListDto>>, axum::http::StatusCode> {
    service::list_by_day(&connection_id, &business_date)
        .await
        .map(|items| Json(items.into_iter().map(|d| d.to_list_dto()).collect()))
        .map_err(|e| {
            tracing::error!(
                "list_by_day a033 {}/{}: {}",
                connection_id,
                business_date,
                e
            );
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/a033/wb-day-close
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_active(
    Json(body): Json<CreateActiveRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    service::create_active(&body.connection_id, &body.business_date)
        .await
        .map(|id| Json(serde_json::json!({ "id": id.to_string() })))
        .map_err(|e| {
            tracing::error!("create a033: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/a033/wb-day-close/:id/recalculate
// ─────────────────────────────────────────────────────────────────────────────

pub async fn recalculate(
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    service::recalculate(uuid).await.map_err(|e| {
        tracing::error!("recalculate a033 {}: {}", id, e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/a033/wb-day-close/:id/repost-problematic-a012
// ─────────────────────────────────────────────────────────────────────────────

pub async fn repost_problematic_a012(
    Path(id): Path<String>,
    Json(body): Json<RepostProblematicRequest>,
) -> Result<Json<RepostResult>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    service::repost_problematic_a012(uuid, &body.only_problem_codes)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("repost a033 {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/a033/wb-day-close/:id/archive-and-recreate
// ─────────────────────────────────────────────────────────────────────────────

pub async fn archive_and_recreate(
    Path(id): Path<String>,
    Json(body): Json<ArchiveAndRecreateRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    service::archive_and_recreate(uuid, body.reason)
        .await
        .map(|new_id| Json(serde_json::json!({ "id": new_id.to_string() })))
        .map_err(|e| {
            tracing::error!("archive_and_recreate a033 {}: {}", id, e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/a033/wb-day-close/compare
// ─────────────────────────────────────────────────────────────────────────────

pub async fn compare(
    Json(body): Json<CompareRequest>,
) -> Result<Json<CompareResponse>, axum::http::StatusCode> {
    let active_uuid =
        Uuid::parse_str(&body.active_id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let archived_uuid =
        Uuid::parse_str(&body.archived_id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    service::compare(active_uuid, archived_uuid)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("compare a033: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })
}
