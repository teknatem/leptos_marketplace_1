//! Бизнес-логика тикетов: валидация, права доступа, генерация номера,
//! работа с вложениями через S3.
//!
//! Правило доступа: пользователь видит и редактирует только свои тикеты,
//! администратор — все. Статус и исполнителя меняет только администратор.

use anyhow::{anyhow, Result};
use chrono::Utc;
use contracts::system::s3::S3FileCategory;
use contracts::system::tickets::{
    CreateTicketCommentRequest, CreateTicketRequest, TicketAttachmentDto, TicketCommentDto,
    TicketDetailsResponse, TicketDto, TicketListResponse, TicketStatus, UpdateTicketRequest,
    TICKET_ORIGIN_BITRIX, TICKET_ORIGIN_LLM_CHAT, TICKET_ORIGIN_SELF,
};
use uuid::Uuid;

use super::{change_token, repository};
use crate::system::s3::service as s3_service;

pub struct Requester {
    pub user_id: String,
    pub is_admin: bool,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn normalize_origin(origin: Option<String>) -> String {
    match origin.as_deref() {
        Some(TICKET_ORIGIN_LLM_CHAT) => TICKET_ORIGIN_LLM_CHAT.to_string(),
        Some(TICKET_ORIGIN_BITRIX) => TICKET_ORIGIN_BITRIX.to_string(),
        _ => TICKET_ORIGIN_SELF.to_string(),
    }
}

fn can_access(requester: &Requester, ticket: &TicketDto) -> bool {
    requester.is_admin || ticket.author_user_id == requester.user_id
}

async fn get_accessible(requester: &Requester, ticket_id: &str) -> Result<TicketDto> {
    let ticket = repository::get_by_id(ticket_id)
        .await?
        .ok_or_else(|| anyhow!("Ticket not found"))?;
    if !can_access(requester, &ticket) {
        return Err(anyhow!("Forbidden"));
    }
    Ok(ticket)
}

pub struct ListParams {
    pub status: Option<String>,
    pub ticket_type: Option<String>,
    pub search: Option<String>,
    pub limit: u64,
    pub offset: u64,
}

pub async fn list(requester: &Requester, params: ListParams) -> Result<TicketListResponse> {
    let filter = repository::TicketListFilter {
        author_user_id: if requester.is_admin {
            None
        } else {
            Some(requester.user_id.clone())
        },
        status: params.status.filter(|s| !s.trim().is_empty()),
        ticket_type: params.ticket_type.filter(|s| !s.trim().is_empty()),
        search: params.search.filter(|s| !s.trim().is_empty()),
        limit: params.limit.clamp(1, 500),
        offset: params.offset,
    };
    let (items, total) = repository::list(&filter).await?;
    Ok(TicketListResponse { items, total })
}

pub async fn create(requester: &Requester, req: CreateTicketRequest) -> Result<TicketDto> {
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Err(anyhow!("Ticket title is required"));
    }

    let number = repository::next_code_number().await?;
    let ts = now();
    let ticket = TicketDto {
        id: Uuid::new_v4().to_string(),
        code: format!("T-{:06}", number),
        title,
        description: req.description,
        ticket_type: req.ticket_type,
        status: TicketStatus::New,
        priority: req.priority,
        deadline: req.deadline.filter(|s| !s.trim().is_empty()),
        author_user_id: requester.user_id.clone(),
        author_username: None,
        // Исполнителя назначает только администратор
        assignee_user_id: if requester.is_admin {
            req.assignee_user_id.filter(|s| !s.trim().is_empty())
        } else {
            None
        },
        assignee_username: None,
        tags: req.tags,
        origin: normalize_origin(req.origin),
        context_page_key: req.context_page_key,
        context_json: req.context_json,
        bitrix_task_id: None,
        bitrix_synced_at: None,
        bitrix_received_at: None,
        created_at: ts.clone(),
        updated_at: ts,
        comment_count: 0,
        attachment_count: 0,
    };

    repository::insert(&ticket).await?;
    change_token::TOKEN.bump();
    repository::get_by_id(&ticket.id)
        .await?
        .ok_or_else(|| anyhow!("Ticket not found after insert"))
}

pub async fn get_details(requester: &Requester, ticket_id: &str) -> Result<TicketDetailsResponse> {
    let ticket = get_accessible(requester, ticket_id).await?;
    let comments = repository::list_comments(ticket_id).await?;
    let attachments = repository::list_attachments(ticket_id).await?;

    Ok(TicketDetailsResponse {
        ticket,
        comments,
        attachments,
    })
}

pub async fn update(
    requester: &Requester,
    ticket_id: &str,
    req: UpdateTicketRequest,
) -> Result<TicketDto> {
    let existing = get_accessible(requester, ticket_id).await?;

    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Err(anyhow!("Ticket title is required"));
    }

    let updated = TicketDto {
        title,
        description: req.description,
        ticket_type: req.ticket_type,
        // Статус и исполнителя меняет только администратор
        status: if requester.is_admin {
            req.status
        } else {
            existing.status
        },
        assignee_user_id: if requester.is_admin {
            req.assignee_user_id.filter(|s| !s.trim().is_empty())
        } else {
            existing.assignee_user_id.clone()
        },
        priority: req.priority,
        deadline: req.deadline.filter(|s| !s.trim().is_empty()),
        tags: req.tags,
        updated_at: now(),
        ..existing
    };

    repository::update(&updated).await?;
    change_token::TOKEN.bump();
    repository::get_by_id(ticket_id)
        .await?
        .ok_or_else(|| anyhow!("Ticket not found after update"))
}

pub async fn delete(requester: &Requester, ticket_id: &str) -> Result<()> {
    get_accessible(requester, ticket_id).await?;
    repository::soft_delete(ticket_id, &now()).await?;
    change_token::TOKEN.bump();
    Ok(())
}

// ============================================================================
// Комментарии
// ============================================================================

pub async fn list_comments(
    requester: &Requester,
    ticket_id: &str,
) -> Result<Vec<TicketCommentDto>> {
    get_accessible(requester, ticket_id).await?;
    Ok(repository::list_comments(ticket_id).await?)
}

pub async fn add_comment(
    requester: &Requester,
    ticket_id: &str,
    req: CreateTicketCommentRequest,
) -> Result<TicketCommentDto> {
    get_accessible(requester, ticket_id).await?;
    let body = req.body.trim().to_string();
    if body.is_empty() {
        return Err(anyhow!("Comment body is required"));
    }

    let ts = now();
    let comment = TicketCommentDto {
        id: Uuid::new_v4().to_string(),
        ticket_id: ticket_id.to_string(),
        author_user_id: requester.user_id.clone(),
        author_username: None,
        body,
        origin: TICKET_ORIGIN_SELF.to_string(),
        bitrix_comment_id: None,
        created_at: ts.clone(),
        attachments: Vec::new(),
    };
    repository::insert_comment(&comment).await?;
    repository::touch(ticket_id, &ts).await?;
    change_token::TOKEN.bump();

    repository::get_comment(&comment.id)
        .await?
        .ok_or_else(|| anyhow!("Comment not found after insert"))
}

// ============================================================================
// Вложения
// ============================================================================

pub async fn upload_attachment(
    requester: &Requester,
    ticket_id: &str,
    comment_id: Option<String>,
    file: s3_service::UploadedFile,
) -> Result<TicketAttachmentDto> {
    get_accessible(requester, ticket_id).await?;

    if let Some(comment_id) = &comment_id {
        let comment = repository::get_comment(comment_id)
            .await?
            .ok_or_else(|| anyhow!("Comment not found"))?;
        if comment.ticket_id != ticket_id {
            return Err(anyhow!("Comment does not belong to this ticket"));
        }
    }

    let filename = file.filename.clone();
    let content_type = file.content_type.clone();
    let file_size = file.bytes.len() as i64;

    let uploaded = s3_service::upload(
        S3FileCategory::TicketAttachments,
        file,
        Some(requester.user_id.clone()),
    )
    .await?;

    let ts = now();
    let attachment = TicketAttachmentDto {
        id: Uuid::new_v4().to_string(),
        ticket_id: ticket_id.to_string(),
        comment_id,
        s3_file_id: uploaded.id.clone(),
        filename,
        content_type,
        file_size,
        created_at: ts.clone(),
    };

    if let Err(error) = repository::insert_attachment(&attachment).await {
        // Не оставляем в S3 объект без записи-ссылки
        let _ = s3_service::delete(&uploaded.id).await;
        return Err(error.into());
    }
    repository::touch(ticket_id, &ts).await?;
    change_token::TOKEN.bump();
    Ok(attachment)
}

pub async fn download_attachment(
    requester: &Requester,
    ticket_id: &str,
    attachment_id: &str,
) -> Result<Option<(TicketAttachmentDto, s3_service::DownloadedFile)>> {
    get_accessible(requester, ticket_id).await?;
    let Some(attachment) = repository::get_attachment(attachment_id).await? else {
        return Ok(None);
    };
    if attachment.ticket_id != ticket_id {
        return Err(anyhow!("Attachment does not belong to this ticket"));
    }
    let Some(download) = s3_service::download(&attachment.s3_file_id).await? else {
        return Ok(None);
    };
    Ok(Some((attachment, download)))
}

pub async fn delete_attachment(
    requester: &Requester,
    ticket_id: &str,
    attachment_id: &str,
) -> Result<()> {
    get_accessible(requester, ticket_id).await?;
    let attachment = repository::get_attachment(attachment_id)
        .await?
        .ok_or_else(|| anyhow!("Attachment not found"))?;
    if attachment.ticket_id != ticket_id {
        return Err(anyhow!("Attachment does not belong to this ticket"));
    }
    let _ = s3_service::delete(&attachment.s3_file_id).await;
    repository::delete_attachment(attachment_id).await?;
    change_token::TOKEN.bump();
    Ok(())
}
