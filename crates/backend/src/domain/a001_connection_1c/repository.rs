use chrono::Utc;
use contracts::domain::a001_connection_1c::aggregate::{
    Connection1CDatabase, Connection1CDatabaseId,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a001_connection_1c_database")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub url: String,
    pub login: String,
    pub password: String,
    pub is_primary: bool,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
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
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        Connection1CDatabase {
            base: BaseAggregate::with_metadata(
                Connection1CDatabaseId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            url: m.url,
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
        a.base
            .description
            .to_lowercase()
            .cmp(&b.base.description.to_lowercase())
    });
    Ok(items)
}

/// Пагинированный список подключений 1C
pub async fn list_paginated(
    limit: u64,
    offset: u64,
    sort_by: &str,
    sort_desc: bool,
) -> anyhow::Result<(Vec<Connection1CDatabase>, u64)> {
    // Подсчет общего количества
    let total = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .count(conn())
        .await?;

    // Получение данных с пагинацией
    let mut query = Entity::find().filter(Column::IsDeleted.eq(false));

    // Сортировка
    query = match sort_by {
        "description" => {
            if sort_desc {
                query.order_by_desc(Column::Description)
            } else {
                query.order_by_asc(Column::Description)
            }
        }
        "url" => {
            if sort_desc {
                query.order_by_desc(Column::Url)
            } else {
                query.order_by_asc(Column::Url)
            }
        }
        "login" => {
            if sort_desc {
                query.order_by_desc(Column::Login)
            } else {
                query.order_by_asc(Column::Login)
            }
        }
        "is_primary" => {
            if sort_desc {
                query.order_by_desc(Column::IsPrimary)
            } else {
                query.order_by_asc(Column::IsPrimary)
            }
        }
        "created_at" => {
            if sort_desc {
                query.order_by_desc(Column::CreatedAt)
            } else {
                query.order_by_asc(Column::CreatedAt)
            }
        }
        _ => {
            // По умолчанию сортировка по description
            if sort_desc {
                query.order_by_desc(Column::Description)
            } else {
                query.order_by_asc(Column::Description)
            }
        }
    };

    let items: Vec<Connection1CDatabase> = query
        .limit(limit)
        .offset(offset)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok((items, total))
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<Connection1CDatabase>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
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

pub async fn insert(aggregate: &Connection1CDatabase) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        url: Set(aggregate.url.clone()),
        login: Set(aggregate.login.clone()),
        password: Set(aggregate.password.clone()),
        is_primary: Set(aggregate.is_primary),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &Connection1CDatabase) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        url: Set(aggregate.url.clone()),
        login: Set(aggregate.login.clone()),
        password: Set(aggregate.password.clone()),
        is_primary: Set(aggregate.is_primary),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
        created_at: sea_orm::ActiveValue::NotSet,
    };
    active.update(conn()).await?;
    Ok(())
}

pub async fn clear_other_primary_flags(except_id: Option<Uuid>) -> anyhow::Result<()> {
    if let Some(current) = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::IsPrimary.eq(true))
        .one(conn())
        .await?
    {
        let should_clear = except_id.map_or(true, |id| id.to_string() != current.id);
        if should_clear {
            let mut clear = ActiveModel::from(current);
            clear.is_primary = Set(false);
            clear.updated_at = Set(Some(Utc::now()));
            clear.update(conn()).await?;
        }
    }
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
