use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::domain::a018_llm_chat;
use crate::domain::a018_llm_chat::job_store::{self, LlmJobStatus};
use contracts::domain::a018_llm_chat::aggregate::{
    LlmChat, LlmChatDetail, LlmChatListItem, LlmChatMessage, ToolTraceEntry,
};
use contracts::domain::a018_llm_chat::context::{AddContextRequest, ContextPackageSummary};

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
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<LlmChatDetail>, axum::http::StatusCode> {
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

/// GET /api/a018-llm-chat/message/:message_id/tool-trace
/// Полный журнал вызовов инструментов для сообщения ассистента.
pub async fn get_tool_trace(
    Path(message_id): Path<String>,
) -> Result<Json<Vec<ToolTraceEntry>>, axum::http::StatusCode> {
    match a018_llm_chat::service::get_tool_trace(&message_id).await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
pub struct SetRatingRequest {
    /// 1..5, либо null чтобы снять оценку.
    pub rating: Option<i32>,
}

/// POST /api/a018-llm-chat/:id/rating
pub async fn set_rating(
    Path(id): Path<String>,
    Json(payload): Json<SetRatingRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match a018_llm_chat::service::set_rating(&id, payload.rating).await {
        Ok(()) => Ok(Json(json!({ "success": true }))),
        Err(e) => {
            tracing::warn!("set_rating failed for chat {}: {}", id, e);
            Err(axum::http::StatusCode::BAD_REQUEST)
        }
    }
}

#[derive(Serialize)]
pub struct SendJobResponse {
    pub job_id: String,
}

#[derive(Serialize)]
pub struct JobStatusResponse {
    pub status: String, // "pending" | "done" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<LlmChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Текущий этап выполнения (только для status == "pending").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<job_store::JobProgress>,
}

/// POST /api/a018-llm-chat/:id/messages
/// Immediately returns 202 Accepted with a job_id.
/// The LLM call runs in background; poll GET /jobs/:job_id for the result.
pub async fn send_message(
    Path(id): Path<String>,
    Json(payload): Json<a018_llm_chat::service::SendMessageRequest>,
) -> Result<(StatusCode, Json<SendJobResponse>), StatusCode> {
    let proposed_job_id = Uuid::new_v4().to_string();
    let request_id = payload
        .request_id
        .clone()
        .unwrap_or_else(|| proposed_job_id.clone());
    let (job_id, should_start) = job_store::register(&proposed_job_id, &id, &request_id)
        .await
        .map_err(|error| {
            tracing::error!("failed to register durable LLM job: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !should_start {
        return Ok((StatusCode::ACCEPTED, Json(SendJobResponse { job_id })));
    }

    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        tracing::info!("[llm_job] started job_id={} chat_id={}", job_id_clone, id);
        match a018_llm_chat::service::send_message(&id, payload, Some(&job_id_clone)).await {
            Ok(msg) => {
                tracing::info!("[llm_job] done job_id={}", job_id_clone);
                job_store::complete(&job_id_clone, msg).await;
            }
            Err(e) => {
                tracing::error!("[llm_job] error job_id={} err={}", job_id_clone, e);
                job_store::fail(&job_id_clone, e.to_string()).await;
            }
        }
    });

    Ok((StatusCode::ACCEPTED, Json(SendJobResponse { job_id })))
}

/// POST /api/a018-llm-chat/jobs/:job_id/cancel
pub async fn cancel_job(Path(job_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    let cancelled = job_store::cancel(&job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if cancelled {
        Ok(Json(json!({ "ok": true })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/a018-llm-chat/jobs/:job_id
/// Returns current status of a background LLM job.
pub async fn poll_job(Path(job_id): Path<String>) -> Result<Json<JobStatusResponse>, StatusCode> {
    match job_store::take(&job_id).await {
        None => Err(StatusCode::NOT_FOUND),
        Some(LlmJobStatus::Pending(progress)) => Ok(Json(JobStatusResponse {
            status: "pending".to_string(),
            message: None,
            error: None,
            progress: Some(progress),
        })),
        Some(LlmJobStatus::Done(msg)) => Ok(Json(JobStatusResponse {
            status: "done".to_string(),
            message: Some(msg),
            error: None,
            progress: None,
        })),
        Some(LlmJobStatus::Error(e)) => Ok(Json(JobStatusResponse {
            status: "error".to_string(),
            message: None,
            error: Some(e),
            progress: None,
        })),
    }
}

/// GET /api/a018-llm-chat/:id/context
/// Список пакетов контекста, привязанных к чату.
pub async fn get_chat_context(
    Path(id): Path<String>,
) -> Result<Json<Vec<ContextPackageSummary>>, StatusCode> {
    match a018_llm_chat::service::list_chat_context(&id).await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/a018-llm-chat-context/:id
/// Получить один пакет контекста (для details-страницы просмотра контекста LLM).
pub async fn get_context_package(
    Path(id): Path<String>,
) -> Result<Json<ContextPackageSummary>, StatusCode> {
    match a018_llm_chat::service::get_context_by_id(&id).await {
        Ok(Some(s)) => Ok(Json(s)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/a018-llm-chat/:id/context
/// Собрать контекст текущей страницы по page_key и привязать к чату.
pub async fn add_chat_context(
    Path(id): Path<String>,
    Json(req): Json<AddContextRequest>,
) -> Result<Json<ContextPackageSummary>, StatusCode> {
    match a018_llm_chat::service::add_chat_context(&id, &req.page_key, req.label.as_deref()).await {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            tracing::error!("add_chat_context error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
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
