use anyhow::Result;
use chrono::Utc;
use contracts::domain::a016_ym_returns::aggregate::{
    YmReturn, YmReturnHeader, YmReturnId, YmReturnLine, YmReturnSourceMeta, YmReturnState,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a016_ym_returns")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub return_id: i64,
    pub order_id: i64,
    pub header_json: String,
    pub lines_json: String,
    pub state_json: String,
    pub source_meta_json: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for YmReturn {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header: YmReturnHeader = serde_json::from_str(&m.header_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize header_json for return_id: {}",
                m.return_id
            )
        });
        let lines: Vec<YmReturnLine> = serde_json::from_str(&m.lines_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize lines_json for return_id: {}",
                m.return_id
            )
        });
        let state: YmReturnState = serde_json::from_str(&m.state_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize state_json for return_id: {}",
                m.return_id
            )
        });
        let source_meta: YmReturnSourceMeta =
            serde_json::from_str(&m.source_meta_json).unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize source_meta_json for return_id: {}",
                    m.return_id
                )
            });

        YmReturn {
            base: BaseAggregate::with_metadata(
                YmReturnId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            header,
            lines,
            state,
            source_meta,
            is_posted: m.is_posted,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> Result<Vec<YmReturn>> {
    let items: Vec<YmReturn> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .order_by_desc(Column::UpdatedAt)
        .limit(1000)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmReturn>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_return_id(return_id: i64) -> Result<Option<YmReturn>> {
    let result = Entity::find()
        .filter(Column::ReturnId.eq(return_id))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn upsert_document(aggregate: &YmReturn) -> Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let existing = get_by_return_id(aggregate.header.return_id).await?;

    let header_json = serde_json::to_string(&aggregate.header)?;
    let lines_json = serde_json::to_string(&aggregate.lines)?;
    let state_json = serde_json::to_string(&aggregate.state)?;
    let source_meta_json = serde_json::to_string(&aggregate.source_meta)?;

    if let Some(existing_doc) = existing {
        let existing_uuid = existing_doc.base.id.value();
        let active = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            return_id: Set(aggregate.header.return_id),
            order_id: Set(aggregate.header.order_id),
            header_json: Set(header_json),
            lines_json: Set(lines_json),
            state_json: Set(state_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.base.metadata.is_posted),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        active.update(conn()).await?;
        Ok(existing_uuid)
    } else {
        let active = ActiveModel {
            id: Set(uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            return_id: Set(aggregate.header.return_id),
            order_id: Set(aggregate.header.order_id),
            header_json: Set(header_json),
            lines_json: Set(lines_json),
            state_json: Set(state_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.base.metadata.is_posted),
            created_at: Set(Some(aggregate.base.metadata.created_at)),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version),
        };
        active.insert(conn()).await?;
        Ok(uuid)
    }
}

pub async fn soft_delete(id: Uuid) -> Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}

