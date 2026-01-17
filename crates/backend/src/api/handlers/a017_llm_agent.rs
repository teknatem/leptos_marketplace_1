use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::a017_llm_agent;
use contracts::domain::a017_llm_agent::aggregate::LlmAgent;

#[derive(Deserialize)]
pub struct LlmAgentListParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Serialize)]
pub struct LlmAgentPaginatedResponse {
    pub items: Vec<LlmAgent>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// GET /api/a017-llm-agent
pub async fn list_all() -> Result<Json<Vec<LlmAgent>>, axum::http::StatusCode> {
    match a017_llm_agent::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a017-llm-agent/list
pub async fn list_paginated(
    Query(params): Query<LlmAgentListParams>,
) -> Result<Json<LlmAgentPaginatedResponse>, axum::http::StatusCode> {
    let limit = params.limit.unwrap_or(100).clamp(10, 10000);
    let offset = params.offset.unwrap_or(0);
    let sort_by = params.sort_by.as_deref().unwrap_or("description");
    let sort_desc = params.sort_desc.unwrap_or(false);

    match a017_llm_agent::service::list_paginated(limit, offset, sort_by, sort_desc).await {
        Ok((items, total)) => {
            let page_size = limit as usize;
            let page = (offset as usize) / page_size;
            let total_pages = ((total as usize) + page_size - 1) / page_size;

            Ok(Json(LlmAgentPaginatedResponse {
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

/// GET /api/a017-llm-agent/:id
pub async fn get_by_id(Path(id): Path<String>) -> Result<Json<LlmAgent>, axum::http::StatusCode> {
    match a017_llm_agent::service::get_by_id(&id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /api/a017-llm-agent/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    match a017_llm_agent::service::delete(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a017-llm-agent
pub async fn upsert(
    Json(dto): Json<a017_llm_agent::service::LlmAgentDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    if dto.id.is_some() {
        // Update
        match a017_llm_agent::service::update(dto).await {
            Ok(_) => Ok(Json(json!({"success": true}))),
            Err(e) => {
                tracing::error!("Failed to update LLM agent: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // Create
        match a017_llm_agent::service::create(dto).await {
            Ok(id) => Ok(Json(json!({"success": true, "id": id.to_string()}))),
            Err(e) => {
                tracing::error!("Failed to create LLM agent: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// GET /api/a017-llm-agent/primary
pub async fn get_primary() -> Result<Json<LlmAgent>, axum::http::StatusCode> {
    match a017_llm_agent::service::get_primary().await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a017-llm-agent/:id/test
/// Тест подключения к LLM провайдеру
pub async fn test_connection(Path(id): Path<String>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    use crate::shared::llm::{openai_provider::OpenAiProvider, LlmProvider};
    use contracts::domain::a017_llm_agent::aggregate::LlmProviderType;

    // Получаем агента
    let agent = match a017_llm_agent::service::get_by_id(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => return Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Создаём провайдер и тестируем подключение
    let result: Result<serde_json::Value, ()> = match agent.provider_type {
        LlmProviderType::OpenAI => {
            let provider = OpenAiProvider::new_with_endpoint(
                agent.api_endpoint.clone(),
                agent.api_key.clone(),
                agent.model_name.clone(),
                agent.temperature,
                agent.max_tokens,
            );

            match provider.test_connection().await {
                Ok(()) => Ok(json!({
                    "success": true,
                    "message": format!("Successfully connected to {} ({})", agent.model_name, agent.provider_type.as_str()),
                    "provider": agent.provider_type.as_str(),
                    "model": agent.model_name
                })),
                Err(e) => Ok(json!({
                    "success": false,
                    "message": format!("Connection failed: {}", e),
                    "provider": agent.provider_type.as_str(),
                    "model": agent.model_name
                })),
            }
        }
        _ => {
            Ok(json!({
                "success": false,
                "message": format!("Provider {} not yet implemented", agent.provider_type.as_str()),
                "provider": agent.provider_type.as_str(),
                "model": agent.model_name
            }))
        }
    };

    match result {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}
