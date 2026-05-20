use contracts::system::history::PageHistoryDto;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, Statement,
};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_page_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub owner_user_id: String,
    pub tab_key: String,
    pub title: String,
    pub opened_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

fn dto_from_model(model: Model) -> PageHistoryDto {
    PageHistoryDto {
        id: model.id,
        owner_user_id: model.owner_user_id,
        tab_key: model.tab_key,
        title: model.title,
        opened_at: model.opened_at,
    }
}

pub async fn list_recent(user_id: &str, limit: u64) -> Result<Vec<PageHistoryDto>, DbErr> {
    let models = Entity::find()
        .filter(Column::OwnerUserId.eq(user_id))
        .order_by_desc(Column::OpenedAt)
        .limit(limit)
        .all(conn())
        .await?;
    Ok(models.into_iter().map(dto_from_model).collect())
}

/// Most recent entry for a user, used to collapse consecutive duplicates.
pub async fn find_latest(user_id: &str) -> Result<Option<PageHistoryDto>, DbErr> {
    let model = Entity::find()
        .filter(Column::OwnerUserId.eq(user_id))
        .order_by_desc(Column::OpenedAt)
        .one(conn())
        .await?;
    Ok(model.map(dto_from_model))
}

pub async fn create(model: Model) -> Result<PageHistoryDto, DbErr> {
    ActiveModel {
        id: Set(model.id.clone()),
        owner_user_id: Set(model.owner_user_id.clone()),
        tab_key: Set(model.tab_key.clone()),
        title: Set(model.title.clone()),
        opened_at: Set(model.opened_at.clone()),
    }
    .insert(conn())
    .await?;
    Ok(dto_from_model(model))
}

/// Update timestamp (and title) of an existing entry — used when re-opening
/// the same page consecutively, so the history shows a single freshest row.
pub async fn touch(id: &str, title: &str, opened_at: &str) -> Result<(), DbErr> {
    conn()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE sys_page_history SET title = ?1, opened_at = ?2 WHERE id = ?3",
            vec![title.into(), opened_at.into(), id.into()],
        ))
        .await?;
    Ok(())
}

/// Keep only the newest `keep` rows for the user.
pub async fn prune(user_id: &str, keep: u64) -> Result<(), DbErr> {
    conn()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM sys_page_history \
             WHERE owner_user_id = ?1 \
               AND id NOT IN ( \
                 SELECT id FROM sys_page_history \
                 WHERE owner_user_id = ?1 \
                 ORDER BY opened_at DESC \
                 LIMIT ?2 \
               )",
            vec![user_id.into(), (keep as i64).into()],
        ))
        .await?;
    Ok(())
}

pub async fn clear(user_id: &str) -> Result<(), DbErr> {
    Entity::delete_many()
        .filter(Column::OwnerUserId.eq(user_id))
        .exec(conn())
        .await?;
    Ok(())
}
