use contracts::system::favorites::FavoriteDto;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, QueryFilter, QueryOrder, Set,
    Statement,
};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_favorites")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub owner_user_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub target_title: String,
    pub tab_key: String,
    pub color: String,
    pub comment: Option<String>,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

fn dto_from_model(model: Model, owner_username: Option<String>) -> FavoriteDto {
    FavoriteDto {
        id: model.id,
        owner_user_id: model.owner_user_id,
        owner_username,
        target_kind: model.target_kind,
        target_id: model.target_id,
        target_title: model.target_title,
        tab_key: model.tab_key,
        color: model.color,
        comment: model.comment,
        is_global: model.is_global,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

async fn username_for_user(user_id: &str) -> Result<Option<String>, DbErr> {
    let row = conn()
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT username FROM sys_users WHERE id = ?1",
            vec![user_id.to_string().into()],
        ))
        .await?;

    Ok(row.and_then(|r| r.try_get::<String>("", "username").ok()))
}

pub async fn list_visible(user_id: &str) -> Result<Vec<FavoriteDto>, DbErr> {
    let models = Entity::find()
        .filter(
            Column::OwnerUserId
                .eq(user_id)
                .or(Column::IsGlobal.eq(true)),
        )
        .order_by_desc(Column::UpdatedAt)
        .all(conn())
        .await?;

    let mut items = Vec::with_capacity(models.len());
    for model in models {
        let owner_username = username_for_user(&model.owner_user_id).await?;
        items.push(dto_from_model(model, owner_username));
    }
    Ok(items)
}

pub async fn find_personal_target(
    owner_user_id: &str,
    target_kind: &str,
    target_id: &str,
) -> Result<Option<FavoriteDto>, DbErr> {
    let model = Entity::find()
        .filter(Column::OwnerUserId.eq(owner_user_id))
        .filter(Column::TargetKind.eq(target_kind))
        .filter(Column::TargetId.eq(target_id))
        .one(conn())
        .await?;

    if let Some(model) = model {
        let owner_username = username_for_user(&model.owner_user_id).await?;
        Ok(Some(dto_from_model(model, owner_username)))
    } else {
        Ok(None)
    }
}

pub async fn find_by_id(id: &str) -> Result<Option<FavoriteDto>, DbErr> {
    let model = Entity::find_by_id(id.to_string()).one(conn()).await?;
    if let Some(model) = model {
        let owner_username = username_for_user(&model.owner_user_id).await?;
        Ok(Some(dto_from_model(model, owner_username)))
    } else {
        Ok(None)
    }
}

pub async fn create(model: Model) -> Result<FavoriteDto, DbErr> {
    let owner_username = username_for_user(&model.owner_user_id).await?;
    ActiveModel {
        id: Set(model.id.clone()),
        owner_user_id: Set(model.owner_user_id.clone()),
        target_kind: Set(model.target_kind.clone()),
        target_id: Set(model.target_id.clone()),
        target_title: Set(model.target_title.clone()),
        tab_key: Set(model.tab_key.clone()),
        color: Set(model.color.clone()),
        comment: Set(model.comment.clone()),
        is_global: Set(model.is_global),
        created_at: Set(model.created_at.clone()),
        updated_at: Set(model.updated_at.clone()),
    }
    .insert(conn())
    .await?;
    Ok(dto_from_model(model, owner_username))
}

pub async fn update(model: Model) -> Result<FavoriteDto, DbErr> {
    let owner_username = username_for_user(&model.owner_user_id).await?;
    ActiveModel {
        id: Set(model.id.clone()),
        owner_user_id: Set(model.owner_user_id.clone()),
        target_kind: Set(model.target_kind.clone()),
        target_id: Set(model.target_id.clone()),
        target_title: Set(model.target_title.clone()),
        tab_key: Set(model.tab_key.clone()),
        color: Set(model.color.clone()),
        comment: Set(model.comment.clone()),
        is_global: Set(model.is_global),
        created_at: Set(model.created_at.clone()),
        updated_at: Set(model.updated_at.clone()),
    }
    .update(conn())
    .await?;
    Ok(dto_from_model(model, owner_username))
}

pub async fn delete(id: &str) -> Result<(), DbErr> {
    Entity::delete_by_id(id.to_string()).exec(conn()).await?;
    Ok(())
}
