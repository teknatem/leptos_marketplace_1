use contracts::domain::connection_1c::aggregate::{
    BaseAggregate, Connection1CDatabase, Connection1CDatabaseId, EntityMetadata,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use sea_orm::entity::prelude::*;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "connection_1c_database")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub description: String,
    pub url: String,
    pub comment: Option<String>,
    pub login: String,
    pub password: String,
    pub is_primary: bool,
    pub is_deleted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for Connection1CDatabase {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            version: 0,
        };
        Connection1CDatabase {
            base: BaseAggregate::with_metadata(Connection1CDatabaseId(m.id), metadata),
            description: m.description,
            url: m.url,
            comment: m.comment,
            login: m.login,
            password: m.password,
            is_primary: m.is_primary,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<Connection1CDatabase>> {
    let mut items: Vec<Connection1CDatabase> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    items.sort_by(|a, b| {
        a.description
            .to_lowercase()
            .cmp(&b.description.to_lowercase())
    });
    Ok(items)
}

pub async fn get_by_id(id: i32) -> anyhow::Result<Option<Connection1CDatabase>> {
    let result = Entity::find_by_id(id).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn get_primary() -> anyhow::Result<Option<Connection1CDatabase>> {
    let result = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::IsPrimary.eq(true))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &Connection1CDatabase) -> anyhow::Result<i32> {
    let active = ActiveModel {
        description: Set(aggregate.description.clone()),
        url: Set(aggregate.url.clone()),
        comment: Set(aggregate.comment.clone()),
        login: Set(aggregate.login.clone()),
        password: Set(aggregate.password.clone()),
        is_primary: Set(aggregate.is_primary),
        is_deleted: Set(false),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        ..Default::default()
    };
    let res = active.insert(conn()).await?;
    Ok(res.id)
}

pub async fn update(aggregate: &Connection1CDatabase) -> anyhow::Result<()> {
    let id = aggregate.base.id.0;
    let active = ActiveModel {
        id: Set(id),
        description: Set(aggregate.description.clone()),
        url: Set(aggregate.url.clone()),
        comment: Set(aggregate.comment.clone()),
        login: Set(aggregate.login.clone()),
        password: Set(aggregate.password.clone()),
        is_primary: Set(aggregate.is_primary),
        is_deleted: Set(false),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        ..Default::default()
    };
    active.update(conn()).await?;
    Ok(())
}

pub async fn clear_other_primary_flags(except_id: Option<i32>) -> anyhow::Result<()> {
    if let Some(current) = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::IsPrimary.eq(true))
        .one(conn())
        .await?
    {
        if except_id.map_or(true, |id| id != current.id) {
            let mut clear = ActiveModel::from(current);
            clear.is_primary = Set(false);
            clear.updated_at = Set(Some(Utc::now()));
            clear.update(conn()).await?;
        }
    }
    Ok(())
}

pub async fn soft_delete(id: i32) -> anyhow::Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}

