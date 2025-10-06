use chrono::Utc;
use contracts::domain::a003_counterparty::aggregate::{Counterparty, CounterpartyId};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a003_counterparty")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub is_folder: bool,
    pub parent_id: Option<String>,
    pub inn: String,
    pub kpp: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for Counterparty {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        Counterparty {
            base: BaseAggregate::with_metadata(
                CounterpartyId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            is_folder: m.is_folder,
            parent_id: m.parent_id,
            inn: m.inn,
            kpp: m.kpp,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<Counterparty>> {
    let mut items: Vec<Counterparty> = Entity::find()
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    // Sort: folders first, then by description (case-insensitive)
    items.sort_by(|a, b| match (a.is_folder, b.is_folder) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .base
            .description
            .to_lowercase()
            .cmp(&b.base.description.to_lowercase()),
    });
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<Counterparty>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &Counterparty) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        is_folder: Set(aggregate.is_folder),
        parent_id: Set(aggregate.parent_id.clone()),
        inn: Set(aggregate.inn.clone()),
        kpp: Set(aggregate.kpp.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &Counterparty) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        is_folder: Set(aggregate.is_folder),
        parent_id: Set(aggregate.parent_id.clone()),
        inn: Set(aggregate.inn.clone()),
        kpp: Set(aggregate.kpp.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
        created_at: sea_orm::ActiveValue::NotSet,
    };
    active.update(conn()).await?;
    Ok(())
}

pub async fn soft_delete(id: Uuid) -> anyhow::Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}
