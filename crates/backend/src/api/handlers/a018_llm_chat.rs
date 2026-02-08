use axum::{
    extract::{Path, Query, Multipart},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::a018_llm_chat;
use contracts::domain::a018_llm_chat::aggregate::{LlmChat, LlmChatMessage, LlmChatListItem};

#[derive(Deserialize)]
pub struct LlmChatListParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Serialize)]
pub struct LlmChatPaginatedResponse {
    pub items: Vec<LlmChat>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// GET /api/a018-llm-chat
pub async fn list_all() -> Result<Json<Vec<LlmChat>>, axum::http::StatusCode> {
    match a018_llm_chat::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a018-llm-chat/with-stats
pub async fn list_with_stats() -> Result<Json<Vec<LlmChatListItem>>, axum::http::StatusCode> {
    match a018_llm_chat::service::list_with_stats().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a018-llm-chat/list
pub async fn list_paginated(
    Query(params): Query<LlmChatListParams>,
) -> Result<Json<LlmChatPaginatedResponse>, axum::http::StatusCode> {
    let limit = params.limit.unwrap_or(100).clamp(10, 10000);
    let offset = params.offset.unwrap_or(0);
    let page = (offset / limit) as u64;

    match a018_llm_chat::service::list_paginated(page, limit).await {
        Ok((items, total)) => {
            let page_size = limit as usize;
            let page = (offset as usize) / page_size;
            let total_pages = ((total as usize) + page_size - 1) / page_size;

            Ok(Json(LlmChatPaginatedResponse {
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

/// GET /api/a018-llm-chat/:id
pub async fn get_by_id(Path(id): Path<String>) -> Result<Json<LlmChat>, axum::http::StatusCode> {
    match a018_llm_chat::service::get_by_id(&id).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /api/a018-llm-chat/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    match a018_llm_chat::service::delete(&id).await {
        Ok(()) => Ok(()),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a018-llm-chat
pub async fn upsert(
    Json(dto): Json<a018_llm_chat::service::LlmChatDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    if dto.id.is_some() {
        // Update
        match a018_llm_chat::service::update(dto).await {
            Ok(_) => Ok(Json(json!({"success": true}))),
            Err(e) => {
                tracing::error!("Failed to update LLM chat: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        // Create
        match a018_llm_chat::service::create(dto).await {
            Ok(id) => Ok(Json(json!({"success": true, "id": id.to_string()}))),
            Err(e) => {
                tracing::error!("Failed to create LLM chat: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// GET /api/a018-llm-chat/:id/messages
pub async fn get_messages(
    Path(id): Path<String>,
) -> Result<Json<Vec<LlmChatMessage>>, axum::http::StatusCode> {
    match a018_llm_chat::service::get_messages(&id).await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a018-llm-chat/:id/messages
pub async fn send_message(
    Path(id): Path<String>,
    Json(payload): Json<a018_llm_chat::service::SendMessageRequest>,
) -> Result<Json<LlmChatMessage>, axum::http::StatusCode> {
    match a018_llm_chat::service::send_message(&id, payload).await {
        Ok(msg) => Ok(Json(msg)),
        Err(e) => {
            tracing::error!("Failed to send LLM message: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub filename: String,
    pub file_size: i64,
}

/// POST /api/a018-llm-chat/:id/upload
pub async fn upload_attachment(
    Path(chat_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, axum::http::StatusCode> {
    match a018_llm_chat::service::upload_attachment(&chat_id, &mut multipart).await {
        Ok(attachment) => Ok(Json(UploadResponse {
            id: attachment.id.to_string(),
            filename: attachment.filename,
            file_size: attachment.file_size,
        })),
        Err(e) => {
            tracing::error!("Failed to upload attachment: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
