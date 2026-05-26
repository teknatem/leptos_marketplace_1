use axum::{
    extract::{Path, Query},
    Json,
};
use contracts::quality::{
    CheckDetails, CheckResult, NipCleanupRequest, NipCleanupResult, NipGroupsResponse,
    NipProjectionRow, NipRepostRequest, NipRepostResult, QualityCheckInfo, QualityCheckSource,
};
use serde::Deserialize;

/// GET /api/quality/checks
pub async fn list_checks() -> Json<Vec<QualityCheckInfo>> {
    Json(crate::quality::list_checks())
}

/// POST /api/quality/checks/:id/run
pub async fn run_check(
    Path(id): Path<String>,
) -> Result<Json<CheckResult>, axum::http::StatusCode> {
    match crate::quality::run_check(&id).await {
        Ok(result) => Ok(Json(result)),
        Err(e) if e.to_string().starts_with("NOT_FOUND:") => {
            tracing::warn!("Quality check not found: '{}'", id);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("Quality check '{}' failed: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/quality/checks/:id/details
pub async fn check_details(
    Path(id): Path<String>,
) -> Result<Json<CheckDetails>, axum::http::StatusCode> {
    match crate::quality::check_details(&id).await {
        Ok(details) => Ok(Json(details)),
        Err(e) if e.to_string().starts_with("NOT_FOUND:") => {
            tracing::warn!("Quality check details not found: '{}'", id);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("check_details '{}': {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/quality/checks/:id/sources
pub async fn list_sources(
    Path(id): Path<String>,
) -> Result<Json<Vec<QualityCheckSource>>, axum::http::StatusCode> {
    match crate::quality::list_check_sources(&id) {
        Ok(sources) => Ok(Json(sources)),
        Err(e) if e.to_string().starts_with("NOT_FOUND:") => {
            tracing::warn!("Quality check sources not found: '{}'", id);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("list_sources '{}': {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GroupsQuery {
    pub projection_table: String,
    #[serde(default)]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    #[serde(default = "default_sort_groups")]
    pub sort_by: String,
    #[serde(default)]
    pub sort_desc: bool,
}

fn default_page_size() -> i64 {
    50
}
fn default_sort_groups() -> String {
    "missing_count".to_string()
}

/// GET /api/quality/checks/:id/groups?projection_table=...&page=0&page_size=50&sort_by=...&sort_desc=false
pub async fn list_groups(
    Path(id): Path<String>,
    Query(q): Query<GroupsQuery>,
) -> Result<Json<NipGroupsResponse>, axum::http::StatusCode> {
    match crate::quality::list_check_groups(
        &id,
        &q.projection_table,
        q.page,
        q.page_size,
        &q.sort_by,
        q.sort_desc,
    )
    .await
    {
        Ok(resp) => Ok(Json(resp)),
        Err(e) if e.to_string().starts_with("NOT_FOUND:") => {
            tracing::warn!("Quality check groups not found: '{}': {}", id, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("list_groups '{}': {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RowsQuery {
    pub projection_table: String,
    pub registrator_ref: String,
}

/// GET /api/quality/checks/:id/rows?projection_table=...&registrator_ref=...
pub async fn list_rows(
    Path(id): Path<String>,
    Query(q): Query<RowsQuery>,
) -> Result<Json<Vec<NipProjectionRow>>, axum::http::StatusCode> {
    match crate::quality::list_check_rows(&id, &q.projection_table, &q.registrator_ref).await {
        Ok(rows) => Ok(Json(rows)),
        Err(e) if e.to_string().starts_with("NOT_FOUND:") => {
            tracing::warn!("Quality check rows not found: '{}': {}", id, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("list_rows '{}': {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/quality/checks/:id/repost
pub async fn bulk_repost(
    Path(id): Path<String>,
    Json(body): Json<NipRepostRequest>,
) -> Result<Json<NipRepostResult>, axum::http::StatusCode> {
    match crate::quality::bulk_repost(&id, &body).await {
        Ok(result) => Ok(Json(result)),
        Err(e) if e.to_string().starts_with("NOT_FOUND:") => {
            tracing::warn!("Quality bulk_repost not found: '{}': {}", id, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("bulk_repost '{}': {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/quality/checks/:id/cleanup
pub async fn cleanup_orphans(
    Path(id): Path<String>,
    Json(body): Json<NipCleanupRequest>,
) -> Result<Json<NipCleanupResult>, axum::http::StatusCode> {
    match crate::quality::cleanup_orphans(&id, &body).await {
        Ok(result) => Ok(Json(result)),
        Err(e) if e.to_string().starts_with("NOT_FOUND:") => {
            tracing::warn!("Quality cleanup not found: '{}': {}", id, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("cleanup_orphans '{}': {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
