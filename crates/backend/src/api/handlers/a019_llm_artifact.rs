use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::a019_llm_artifact;
use contracts::domain::a019_llm_artifact::aggregate::LlmArtifact;

#[derive(Deserialize)]
pub struct LlmArtifactListParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Serialize)]
pub struct LlmArtifactPaginatedResponse {
    pub items: Vec<LlmArtifact>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// GET /api/a019-llm-artifact
pub async fn list_all() -> Result<Json<Vec<LlmArtifact>>, axum::http::StatusCode> {
    match a019_llm_artifact::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a019-llm-artifact/list
pub async fn list_paginated(
    Query(params): Query<LlmArtifactListParams>,
) -> Result<Json<LlmArtifactPaginatedResponse>, axum::http::StatusCode> {
    let limit = params.limit.unwrap_or(100).clamp(10, 10000);
    let offset = params.offset.unwrap_or(0);
    let page = (offset / limit) as u64;

    match a019_llm_artifact::service::list_paginated(page, limit).await {
        Ok((items, total)) => {
            let page_size = limit as usize;
            let page = (offset as usize) / page_size;
            let total_pages = ((total as usize) + page_size - 1) / page_size;

            Ok(Json(LlmArtifactPaginatedResponse {
                items,
                total,
                page,
                page_size,
                total_pages,
            }))
        }
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a019-llm-artifact/chat/:chat_id
pub async fn list_by_chat(
    Path(chat_id): Path<String>,
) -> Result<Json<Vec<LlmArtifact>>, axum::http::StatusCode> {
    match a019_llm_artifact::service::list_by_chat_id(&chat_id).await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a019-llm-artifact/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<LlmArtifact>, axum::http::StatusCode> {
    match a019_llm_artifact::service::get_by_id(&id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /api/a019-llm-artifact/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    match a019_llm_artifact::service::delete(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a019-llm-artifact
pub async fn upsert(
    Json(dto): Json<a019_llm_artifact::service::LlmArtifactDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    if dto.id.is_some() {
        // Update
        match a019_llm_artifact::service::update(dto).await {
            Ok(_) => Ok(Json(json!({"success": true}))),
            Err(e) => {
                tracing::error!("Failed to update LLM artifact: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // Create
        match a019_llm_artifact::service::create(dto).await {
            Ok(id) => Ok(Json(json!({"success": true, "id": id.to_string()}))),
            Err(e) => {
                tracing::error!("Failed to create LLM artifact: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
