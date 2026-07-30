use axum::body::Body;
use axum::extract::{Multipart, Path, Query};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderValue, Response, StatusCode};
use axum::Json;
use contracts::system::tickets::{
    CreateTicketCommentRequest, CreateTicketRequest, TicketAttachmentUploadResponse,
    TicketCommentDto, TicketDetailsResponse, TicketDto, TicketListResponse, UpdateTicketRequest,
};
use serde::Deserialize;

use crate::system::auth::extractor::CurrentUser;
use crate::system::s3::service::UploadedFile;
use crate::system::tickets::service::{self, ListParams, Requester};

fn requester(claims: &contracts::system::auth::TokenClaims) -> Requester {
    Requester {
        user_id: claims.sub.clone(),
        is_admin: claims.is_admin,
    }
}

fn map_error(err: anyhow::Error) -> StatusCode {
    let message = err.to_string();
    if message.contains("Forbidden") {
        StatusCode::FORBIDDEN
    } else if message.contains("not found") {
        StatusCode::NOT_FOUND
    } else if message.contains("required") || message.contains("does not belong") {
        StatusCode::BAD_REQUEST
    } else if message.contains("S3 storage is disabled") || message.contains("[s3].") {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        tracing::error!("Tickets API error: {}", message);
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub ticket_type: Option<String>,
    pub search: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

pub async fn list(
    CurrentUser(claims): CurrentUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<TicketListResponse>, StatusCode> {
    service::list(
        &requester(&claims),
        ListParams {
            status: query.status,
            ticket_type: query.ticket_type,
            search: query.search,
            limit: query.limit.unwrap_or(100),
            offset: query.offset.unwrap_or(0),
        },
    )
    .await
    .map(Json)
    .map_err(map_error)
}

pub async fn create(
    CurrentUser(claims): CurrentUser,
    Json(req): Json<CreateTicketRequest>,
) -> Result<Json<TicketDto>, StatusCode> {
    service::create(&requester(&claims), req)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn get_details(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<TicketDetailsResponse>, StatusCode> {
    service::get_details(&requester(&claims), &id)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn update(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateTicketRequest>,
) -> Result<Json<TicketDto>, StatusCode> {
    service::update(&requester(&claims), &id, req)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn delete(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    service::delete(&requester(&claims), &id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(map_error)
}

// ============================================================================
// Комментарии
// ============================================================================

pub async fn list_comments(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<TicketCommentDto>>, StatusCode> {
    service::list_comments(&requester(&claims), &id)
        .await
        .map(Json)
        .map_err(map_error)
}

pub async fn add_comment(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
    Json(req): Json<CreateTicketCommentRequest>,
) -> Result<Json<TicketCommentDto>, StatusCode> {
    service::add_comment(&requester(&claims), &id, req)
        .await
        .map(Json)
        .map_err(map_error)
}

// ============================================================================
// Вложения
// ============================================================================

pub async fn upload_attachment(
    CurrentUser(claims): CurrentUser,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<TicketAttachmentUploadResponse>, StatusCode> {
    let mut comment_id: Option<String> = None;
    let mut upload: Option<UploadedFile> = None;

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        tracing::warn!("Failed to read ticket multipart field: {}", err);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or_default().to_string();
        if name == "comment_id" {
            let value = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            if !value.trim().is_empty() {
                comment_id = Some(value.trim().to_string());
            }
            continue;
        }
        if name == "file" {
            let filename = field
                .file_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| "file".to_string());
            let content_type = field.content_type().map(ToString::to_string);
            let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            upload = Some(UploadedFile {
                filename,
                content_type,
                bytes,
            });
        }
    }

    let upload = upload.ok_or(StatusCode::BAD_REQUEST)?;
    service::upload_attachment(&requester(&claims), &id, comment_id, upload)
        .await
        .map(|attachment| Json(TicketAttachmentUploadResponse { attachment }))
        .map_err(map_error)
}

pub async fn download_attachment(
    CurrentUser(claims): CurrentUser,
    Path((id, attachment_id)): Path<(String, String)>,
) -> Result<Response<Body>, StatusCode> {
    let Some((attachment, download)) =
        service::download_attachment(&requester(&claims), &id, &attachment_id)
            .await
            .map_err(map_error)?
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let filename = crate::system::s3::service::sanitize_filename(&attachment.filename);
    let mut response = Response::new(Body::from(download.bytes));
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(
            download
                .content_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(response)
}

pub async fn delete_attachment(
    CurrentUser(claims): CurrentUser,
    Path((id, attachment_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    service::delete_attachment(&requester(&claims), &id, &attachment_id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(map_error)
}
