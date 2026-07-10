use anyhow::Result;
use bytes::Bytes;
use chrono::{Datelike, Utc};
use contracts::system::s3::{S3FileCategory, S3FileDto};
use uuid::Uuid;

use super::{client, repository};
use crate::shared::config;

pub struct UploadedFile {
    pub filename: String,
    pub content_type: Option<String>,
    pub bytes: Bytes,
}

pub struct DownloadedFile {
    pub file: S3FileDto,
    pub bytes: Bytes,
    pub content_type: Option<String>,
}

pub(crate) fn s3_config() -> Result<config::S3Config> {
    let cfg = config::load_config()?;
    cfg.s3.validate_ready()?;
    Ok(cfg.s3)
}

pub fn sanitize_filename(filename: &str) -> String {
    let candidate = filename
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches(['.', '_', '-', ' '])
        .to_string();

    if candidate.is_empty() {
        "file".to_string()
    } else {
        candidate.chars().take(180).collect()
    }
}

pub fn build_object_key(category: &S3FileCategory, id: Uuid, filename: &str) -> String {
    let now = Utc::now();
    format!(
        "{}/{:04}/{:02}/{}/{}",
        category.as_str(),
        now.year(),
        now.month(),
        id,
        sanitize_filename(filename)
    )
}

pub async fn list(category: Option<S3FileCategory>) -> Result<Vec<S3FileDto>> {
    repository::list(category).await
}

pub async fn upload(
    category: S3FileCategory,
    file: UploadedFile,
    uploaded_by_user_id: Option<String>,
) -> Result<S3FileDto> {
    let cfg = s3_config()?;
    let size = file.bytes.len() as u64;
    if size == 0 {
        return Err(anyhow::anyhow!("Uploaded file is empty"));
    }
    if size > cfg.max_upload_bytes() {
        return Err(anyhow::anyhow!(
            "Uploaded file exceeds configured limit of {} MB",
            cfg.max_upload_mb
        ));
    }

    let id = Uuid::new_v4();
    let object_key = build_object_key(&category, id, &file.filename);
    let etag = client::put_object(
        &cfg,
        &object_key,
        file.content_type.as_deref(),
        file.bytes.clone(),
    )
    .await?;

    let now = Utc::now().to_rfc3339();
    let dto = S3FileDto {
        id: id.to_string(),
        category,
        bucket: cfg.bucket.clone(),
        object_key,
        original_filename: file.filename,
        content_type: file.content_type,
        size_bytes: size as i64,
        etag,
        uploaded_by_user_id,
        created_at: now.clone(),
        updated_at: now,
    };

    repository::insert(&dto).await?;
    Ok(dto)
}

pub async fn download(id: &str) -> Result<Option<DownloadedFile>> {
    let Some(file) = repository::get_active_by_id(id).await? else {
        return Ok(None);
    };
    let cfg = s3_config()?;
    let object = client::get_object(&cfg, &file.object_key).await?;
    Ok(Some(DownloadedFile {
        content_type: object.content_type.or_else(|| file.content_type.clone()),
        bytes: object.bytes,
        file,
    }))
}

pub async fn delete(id: &str) -> Result<bool> {
    let Some(file) = repository::get_active_by_id(id).await? else {
        return Ok(false);
    };
    let cfg = s3_config()?;
    client::delete_object(&cfg, &file.object_key).await?;
    let deleted_at = Utc::now().to_rfc3339();
    repository::soft_delete(id, &deleted_at).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_keeps_safe_ascii() {
        assert_eq!(
            sanitize_filename("report-01_final.pdf"),
            "report-01_final.pdf"
        );
    }

    #[test]
    fn sanitize_filename_replaces_unsafe_chars() {
        assert_eq!(
            sanitize_filename("my report/2026.pdf"),
            "my_report_2026.pdf"
        );
        assert_eq!(sanitize_filename("..."), "file");
    }

    #[test]
    fn object_key_uses_category_uuid_and_filename() {
        let id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let key = build_object_key(&S3FileCategory::Documents, id, "doc one.pdf");
        assert!(key.starts_with("documents/"));
        assert!(key.contains("/00000000-0000-0000-0000-000000000001/"));
        assert!(key.ends_with("/doc_one.pdf"));
    }
}
