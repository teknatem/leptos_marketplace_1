use axum::body::Body;
use axum::extract::{Multipart, Path, Query};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderValue, Response, StatusCode};
use axum::Json;
use contracts::system::s3::{S3FileCategory, S3FileListResponse, S3UploadResponse};
use serde::Deserialize;

use crate::system::auth::extractor::CurrentUser;
use crate::system::s3::service::{self, UploadedFile};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
}

fn is_s3_config_error(err: &anyhow::Error) -> bool {
    let text = err.to_string();
    text.contains("S3 storage is disabled")
        || text.contains("[s3].bucket")
        || text.contains("[s3].access_key_id")
        || text.contains("[s3].secret_access_key")
}

fn map_error(err: anyhow::Error) -> StatusCode {
    if is_s3_config_error(&err) {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        tracing::error!("S3 operation failed: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub async fn list_files(
    Query(query): Query<ListQuery>,
) -> Result<Json<S3FileListResponse>, StatusCode> {
    let category = query
        .category
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(S3FileCategory::from);

    let items = service::list(category).await.map_err(map_error)?;
    Ok(Json(S3FileListResponse {
        total: items.len() as u64,
        items,
    }))
}

pub async fn upload_file(
    CurrentUser(claims): CurrentUser,
    mut multipart: Multipart,
) -> Result<Json<S3UploadResponse>, StatusCode> {
    let mut category = S3FileCategory::Documents;
    let mut upload: Option<UploadedFile> = None;

    while let Some(field) = multipart.next_field().await.map_err(|err| {
        tracing::warn!("Failed to read S3 multipart field: {}", err);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or_default().to_string();
        if name == "category" {
            let value = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            category = S3FileCategory::from(value.as_str());
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
    let file = service::upload(category, upload, Some(claims.sub))
        .await
        .map_err(map_error)?;
    Ok(Json(S3UploadResponse { file }))
}

pub async fn download_file(Path(id): Path<String>) -> Result<Response<Body>, StatusCode> {
    let Some(download) = service::download(&id).await.map_err(map_error)? else {
        return Err(StatusCode::NOT_FOUND);
    };

    let filename = service::sanitize_filename(&download.file.original_filename);
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

pub async fn delete_file(Path(id): Path<String>) -> Result<(), StatusCode> {
    let deleted = service::delete(&id).await.map_err(map_error)?;
    if deleted {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
