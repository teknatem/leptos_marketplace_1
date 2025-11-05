use anyhow::Result;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::db::get_connection;

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
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

/// Сохранить сырой JSON ответ от API маркетплейса
/// Возвращает уникальный ref (id записи) для использования в source_ref
pub async fn save_raw_json(
    marketplace: &str,
    document_type: &str,
    document_no: &str,
    raw_json: &str,
    fetched_at: chrono::DateTime<Utc>,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    
    let active = ActiveModel {
        id: Set(id.clone()),
        marketplace: Set(marketplace.to_string()),
        document_type: Set(document_type.to_string()),
        document_no: Set(document_no.to_string()),
        raw_json: Set(raw_json.to_string()),
        fetched_at: Set(fetched_at.to_rfc3339()),
        created_at: Set(Utc::now().to_rfc3339()),
    };
    
    active.insert(conn()).await?;
    
    tracing::debug!(
        "Saved raw JSON: marketplace={}, document_type={}, document_no={}, id={}",
        marketplace,
        document_type,
        document_no,
        id
    );
    
    Ok(id)
}

/// Получить сырой JSON по ref
pub async fn get_by_ref(ref_id: &str) -> Result<Option<String>> {
    let result = Entity::find_by_id(ref_id.to_string())
        .one(conn())
        .await?;
    
    Ok(result.map(|m| m.raw_json))
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
        .one(conn())
        .await?;
    
    Ok(result)
}

/// Удалить старые записи (старше N дней)
pub async fn cleanup_old(days: i64) -> Result<u64> {
    let cutoff_date = Utc::now() - chrono::Duration::days(days);
    let cutoff_str = cutoff_date.to_rfc3339();
    
    let result = Entity::delete_many()
        .filter(Column::CreatedAt.lt(cutoff_str))
        .exec(conn())
        .await?;
    
    tracing::info!("Cleaned up {} old raw JSON records (older than {} days)", result.rows_affected, days);
    
    Ok(result.rows_affected)
}
