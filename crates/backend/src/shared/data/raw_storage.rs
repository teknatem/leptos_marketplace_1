use anyhow::Result;
use chrono::Utc;
use contracts::system::raw_storage::{
    DbVacuumResult, DbVacuumStatus, RawStorageCleanupMode, RawStorageCleanupPreview,
    RawStorageCleanupRequest, RawStorageStatus, RawStorageTypeStat,
};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, QueryFilter, QueryOrder, Set,
    Statement,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

use super::db::get_connection;

static RAW_CAPTURE_LOADED: AtomicBool = AtomicBool::new(false);
static RAW_CAPTURE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Модель для хранения сырых JSON от маркетплейсов
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "document_raw_storage")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub marketplace: String,
    pub document_type: String,
    pub document_no: String,
    pub raw_json: String,
    pub fetched_at: String,
    pub created_at: String,
    pub raw_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub fn set_capture_enabled_cache(enabled: bool) {
    RAW_CAPTURE_ENABLED.store(enabled, Ordering::Relaxed);
    RAW_CAPTURE_LOADED.store(true, Ordering::Release);
}

async fn capture_enabled() -> Result<bool> {
    if RAW_CAPTURE_LOADED.load(Ordering::Acquire) {
        return Ok(RAW_CAPTURE_ENABLED.load(Ordering::Relaxed));
    }

    let value =
        crate::system::settings::repository::get_setting("raw_json_capture_enabled").await?;
    let enabled = value.map_or(false, |v| v == "true");
    set_capture_enabled_cache(enabled);
    Ok(enabled)
}

fn raw_hash(raw_json: &str) -> String {
    format!("{:x}", Sha256::digest(raw_json.as_bytes()))
}

/// Сохранить сырой JSON ответ от API маркетплейса
/// Возвращает уникальный ref (id записи) для использования в source_ref
pub async fn save_raw_json(
    marketplace: &str,
    document_type: &str,
    document_no: &str,
    raw_json: &str,
    fetched_at: chrono::DateTime<Utc>,
) -> Result<Option<String>> {
    if !capture_enabled().await? {
        return Ok(None);
    }

    let raw_hash = raw_hash(raw_json);
    if let Some(existing) = Entity::find()
        .filter(Column::Marketplace.eq(marketplace))
        .filter(Column::DocumentType.eq(document_type))
        .filter(Column::DocumentNo.eq(document_no))
        .filter(Column::RawHash.eq(&raw_hash))
        .order_by_desc(Column::CreatedAt)
        .one(conn())
        .await?
    {
        return Ok(Some(existing.id));
    }

    let id = Uuid::new_v4().to_string();

    let active = ActiveModel {
        id: Set(id.clone()),
        marketplace: Set(marketplace.to_string()),
        document_type: Set(document_type.to_string()),
        document_no: Set(document_no.to_string()),
        raw_json: Set(raw_json.to_string()),
        fetched_at: Set(fetched_at.to_rfc3339()),
        created_at: Set(Utc::now().to_rfc3339()),
        raw_hash: Set(raw_hash),
    };

    active.insert(conn()).await?;

    tracing::debug!(
        "Saved raw JSON: marketplace={}, document_type={}, document_no={}, id={}",
        marketplace,
        document_type,
        document_no,
        id
    );

    Ok(Some(id))
}

/// Получить сырой JSON по ref
pub async fn get_by_ref(ref_id: &str) -> Result<Option<String>> {
    if ref_id.trim().is_empty() {
        return Ok(None);
    }

    let result = Entity::find_by_id(ref_id.to_string()).one(conn()).await?;

    Ok(result.map(|m| m.raw_json))
}

pub async fn get_json_value_by_ref(ref_id: &str) -> Result<serde_json::Value> {
    match get_by_ref(ref_id).await? {
        Some(raw_json) => serde_json::from_str(&raw_json).map_err(Into::into),
        None => Ok(serde_json::json!({
            "raw_not_available": true,
            "message": "Raw JSON не сохранен. Включите debug capture в системном разделе Raw JSON, если нужно сохранять API payload."
        })),
    }
}

/// Получить сырой JSON по ключу (marketplace, document_type, document_no)
pub async fn get_by_key(
    marketplace: &str,
    document_type: &str,
    document_no: &str,
) -> Result<Option<Model>> {
    let result = Entity::find()
        .filter(Column::Marketplace.eq(marketplace))
        .filter(Column::DocumentType.eq(document_type))
        .filter(Column::DocumentNo.eq(document_no))
        .order_by_desc(Column::CreatedAt)
        .one(conn())
        .await?;

    Ok(result)
}

const REFERENCED_REFS_CTE: &str = r#"
WITH refs(ref) AS (
    SELECT json_extract(source_meta_json, '$.raw_payload_ref') FROM a010_ozon_fbs_posting
    UNION ALL SELECT json_extract(source_meta_json, '$.raw_payload_ref') FROM a011_ozon_fbo_posting
    UNION ALL SELECT json_extract(source_meta_json, '$.raw_payload_ref') FROM a012_wb_sales
    UNION ALL SELECT json_extract(source_meta_json, '$.raw_payload_ref') FROM a013_ym_order
    UNION ALL SELECT json_extract(source_meta_json, '$.raw_payload_ref') FROM a015_wb_orders
    UNION ALL SELECT json_extract(source_meta_json, '$.marketplace_raw_payload_ref') FROM a015_wb_orders
    UNION ALL SELECT json_extract(source_meta_json, '$.raw_payload_ref') FROM a016_ym_returns
    UNION ALL SELECT raw_payload_ref FROM a020_wb_promotion
    UNION ALL SELECT json_extract(source_meta_json, '$.raw_payload_ref') FROM a029_wb_supply
),
clean_refs(ref) AS (
    SELECT DISTINCT ref FROM refs WHERE ref IS NOT NULL AND ref <> ''
)
"#;

fn cleanup_where(req: &RawStorageCleanupRequest) -> Result<String> {
    match req.mode {
        RawStorageCleanupMode::Unreferenced => {
            Ok("id NOT IN (SELECT ref FROM clean_refs)".to_string())
        }
        RawStorageCleanupMode::All => Ok("1 = 1".to_string()),
        RawStorageCleanupMode::OlderThanDays => {
            let days = req
                .older_than_days
                .ok_or_else(|| anyhow::anyhow!("older_than_days is required"))?;
            if days < 0 {
                anyhow::bail!("older_than_days must be non-negative");
            }
            let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
            Ok(format!(
                "created_at < '{}' AND id NOT IN (SELECT ref FROM clean_refs)",
                cutoff.replace('\'', "''")
            ))
        }
        RawStorageCleanupMode::Duplicates => Ok("id IN (
                SELECT id FROM (
                    SELECT
                        id,
                        row_number() OVER (
                            PARTITION BY marketplace, document_type, document_no,
                                         CASE WHEN raw_hash <> '' THEN raw_hash ELSE raw_json END
                            ORDER BY
                                CASE WHEN id IN (SELECT ref FROM clean_refs) THEN 0 ELSE 1 END,
                                created_at DESC,
                                id DESC
                        ) AS rn
                    FROM document_raw_storage
                )
                WHERE rn > 1
            )
            AND id NOT IN (SELECT ref FROM clean_refs)"
            .to_string()),
    }
}

pub async fn status() -> Result<RawStorageStatus> {
    let conn = conn();
    let capture_enabled = capture_enabled().await?;

    let total_row = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS rows, COALESCE(SUM(length(raw_json)), 0) AS bytes FROM document_raw_storage"
                .to_string(),
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("raw storage total query returned no row"))?;
    let total_rows: i64 = total_row.try_get("", "rows")?;
    let total_bytes: i64 = total_row.try_get("", "bytes")?;

    let referenced_sql = format!(
        "{} SELECT COUNT(*) AS rows FROM document_raw_storage WHERE id IN (SELECT ref FROM clean_refs)",
        REFERENCED_REFS_CTE
    );
    let referenced_row = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            referenced_sql,
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("raw storage referenced query returned no row"))?;
    let referenced_rows: i64 = referenced_row.try_get("", "rows")?;

    let by_type_rows = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT marketplace, document_type, COUNT(*) AS rows, COALESCE(SUM(length(raw_json)), 0) AS bytes
             FROM document_raw_storage
             GROUP BY marketplace, document_type
             ORDER BY bytes DESC"
                .to_string(),
        ))
        .await?;

    let mut by_type = Vec::with_capacity(by_type_rows.len());
    for row in by_type_rows {
        let rows: i64 = row.try_get("", "rows")?;
        let bytes: i64 = row.try_get("", "bytes")?;
        by_type.push(RawStorageTypeStat {
            marketplace: row.try_get("", "marketplace")?,
            document_type: row.try_get("", "document_type")?,
            rows: rows.max(0) as u64,
            raw_mb: bytes as f64 / 1024.0 / 1024.0,
        });
    }

    Ok(RawStorageStatus {
        capture_enabled,
        total_rows: total_rows.max(0) as u64,
        total_mb: total_bytes as f64 / 1024.0 / 1024.0,
        referenced_rows: referenced_rows.max(0) as u64,
        unreferenced_rows: total_rows.saturating_sub(referenced_rows).max(0) as u64,
        by_type,
    })
}

pub async fn cleanup_preview(req: &RawStorageCleanupRequest) -> Result<RawStorageCleanupPreview> {
    let where_sql = cleanup_where(req)?;
    let sql = format!(
        "{} SELECT COUNT(*) AS rows, COALESCE(SUM(length(raw_json)), 0) AS bytes
         FROM document_raw_storage
         WHERE {}",
        REFERENCED_REFS_CTE, where_sql
    );
    let row = conn()
        .query_one(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await?
        .ok_or_else(|| anyhow::anyhow!("raw storage cleanup preview returned no row"))?;
    let rows: i64 = row.try_get("", "rows")?;
    let bytes: i64 = row.try_get("", "bytes")?;
    Ok(RawStorageCleanupPreview {
        rows_to_delete: rows.max(0) as u64,
        estimated_mb: bytes as f64 / 1024.0 / 1024.0,
    })
}

pub async fn cleanup(req: &RawStorageCleanupRequest) -> Result<RawStorageCleanupPreview> {
    let preview = cleanup_preview(req).await?;
    if preview.rows_to_delete == 0 {
        return Ok(preview);
    }

    let where_sql = cleanup_where(req)?;
    let sql = format!(
        "{} DELETE FROM document_raw_storage WHERE {}",
        REFERENCED_REFS_CTE, where_sql
    );
    conn()
        .execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await?;
    Ok(preview)
}

async fn pragma_i64(pragma: &str, column: &str) -> Result<i64> {
    let row = conn()
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            pragma.to_string(),
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("pragma {pragma} returned no row"))?;
    Ok(row.try_get("", column)?)
}

/// Текущий размер файла БД и объём, реально освобождаемый VACUUM (свободные
/// страницы копятся от DELETE/UPDATE во ВСЕХ таблицах, не только document_raw_storage).
pub async fn vacuum_status() -> Result<DbVacuumStatus> {
    let page_count = pragma_i64("PRAGMA page_count", "page_count").await?;
    let freelist_count = pragma_i64("PRAGMA freelist_count", "freelist_count").await?;
    let page_size = pragma_i64("PRAGMA page_size", "page_size").await?;

    Ok(DbVacuumStatus {
        file_mb: (page_count * page_size) as f64 / 1024.0 / 1024.0,
        reclaimable_mb: (freelist_count * page_size) as f64 / 1024.0 / 1024.0,
    })
}

/// Выполнить VACUUM. Пересобирает весь файл БД: держит запись занятой на
/// время выполнения (другие писатели встанут в очередь на busy_timeout),
/// поэтому вызывать нужно вне пиковой нагрузки ("maintenance-окно").
pub async fn vacuum() -> Result<DbVacuumResult> {
    let before = vacuum_status().await?;
    let started = std::time::Instant::now();

    conn()
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "VACUUM".to_string(),
        ))
        .await?;

    let duration_ms = started.elapsed().as_millis() as u64;
    let after = vacuum_status().await?;

    Ok(DbVacuumResult {
        file_mb_before: before.file_mb,
        file_mb_after: after.file_mb,
        freed_mb: (before.file_mb - after.file_mb).max(0.0),
        duration_ms,
    })
}
