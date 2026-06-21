use anyhow::Result;
use contracts::system::s3::{S3FileCategory, S3FileDto};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use crate::shared::data::db::get_connection;

fn row_to_file(row: &sea_orm::QueryResult) -> Result<S3FileDto> {
    let category: String = row.try_get("", "category")?;
    Ok(S3FileDto {
        id: row.try_get("", "id")?,
        category: S3FileCategory::from(category.as_str()),
        bucket: row.try_get("", "bucket")?,
        object_key: row.try_get("", "object_key")?,
        original_filename: row.try_get("", "original_filename")?,
        content_type: row.try_get("", "content_type")?,
        size_bytes: row.try_get("", "size_bytes")?,
        etag: row.try_get("", "etag")?,
        uploaded_by_user_id: row.try_get("", "uploaded_by_user_id")?,
        created_at: row.try_get("", "created_at")?,
        updated_at: row.try_get("", "updated_at")?,
    })
}

pub async fn insert(file: &S3FileDto) -> Result<()> {
    let conn = get_connection();
    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "INSERT INTO sys_files_s3 (
            id, category, bucket, object_key, original_filename, content_type, size_bytes,
            etag, uploaded_by_user_id, created_at, updated_at, is_deleted
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
        [
            file.id.clone().into(),
            file.category.as_str().to_string().into(),
            file.bucket.clone().into(),
            file.object_key.clone().into(),
            file.original_filename.clone().into(),
            file.content_type.clone().into(),
            file.size_bytes.into(),
            file.etag.clone().into(),
            file.uploaded_by_user_id.clone().into(),
            file.created_at.clone().into(),
            file.updated_at.clone().into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn list(category: Option<S3FileCategory>) -> Result<Vec<S3FileDto>> {
    let conn = get_connection();
    let rows = if let Some(category) = category {
        conn.query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id, category, bucket, object_key, original_filename, content_type, size_bytes,
                    etag, uploaded_by_user_id, created_at, updated_at
             FROM sys_files_s3
             WHERE is_deleted = 0 AND category = ?
             ORDER BY created_at DESC",
            [category.as_str().to_string().into()],
        ))
        .await?
    } else {
        conn.query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id, category, bucket, object_key, original_filename, content_type, size_bytes,
                    etag, uploaded_by_user_id, created_at, updated_at
             FROM sys_files_s3
             WHERE is_deleted = 0
             ORDER BY created_at DESC"
                .to_string(),
        ))
        .await?
    };

    rows.iter().map(row_to_file).collect()
}

pub async fn get_active_by_id(id: &str) -> Result<Option<S3FileDto>> {
    let conn = get_connection();
    let row = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id, category, bucket, object_key, original_filename, content_type, size_bytes,
                    etag, uploaded_by_user_id, created_at, updated_at
             FROM sys_files_s3
             WHERE id = ? AND is_deleted = 0",
            [id.into()],
        ))
        .await?;

    row.as_ref().map(row_to_file).transpose()
}

pub async fn soft_delete(id: &str, deleted_at: &str) -> Result<bool> {
    let conn = get_connection();
    let result = conn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE sys_files_s3
             SET is_deleted = 1, deleted_at = ?, updated_at = ?
             WHERE id = ? AND is_deleted = 0",
            [
                deleted_at.to_string().into(),
                deleted_at.to_string().into(),
                id.into(),
            ],
        ))
        .await?;

    Ok(result.rows_affected() > 0)
}
