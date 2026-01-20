use chrono::Utc;
use contracts::domain::a017_llm_agent::aggregate::{LlmAgent, LlmAgentId, LlmProviderType};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a017_llm_agent")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub provider_type: String,
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub system_prompt: Option<String>,
    pub is_primary: bool,
    pub available_models: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for LlmAgent {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let provider_type = LlmProviderType::from_str(&m.provider_type)
            .unwrap_or(LlmProviderType::OpenAI);

        LlmAgent {
            base: BaseAggregate::with_metadata(
                LlmAgentId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            provider_type,
            api_endpoint: m.api_endpoint,
            api_key: m.api_key,
            model_name: m.model_name,
            temperature: m.temperature,
            max_tokens: m.max_tokens,
            system_prompt: m.system_prompt,
            is_primary: m.is_primary,
            available_models: m.available_models,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<LlmAgent>> {
    let mut items: Vec<LlmAgent> = Entity::find()
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

/// Пагинированный список агентов LLM
pub async fn list_paginated(
    limit: u64,
    offset: u64,
    sort_by: &str,
    sort_desc: bool,
) -> anyhow::Result<(Vec<LlmAgent>, u64)> {
    // Подсчет общего количества
    let total = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .count(conn())
        .await?;

    // Получение данных с пагинацией
    let mut query = Entity::find().filter(Column::IsDeleted.eq(false));

    // Сортировка
    query = match sort_by {
        "code" => {
            if sort_desc {
                query.order_by_desc(Column::Code)
            } else {
                query.order_by_asc(Column::Code)
            }
        }
        "description" => {
            if sort_desc {
                query.order_by_desc(Column::Description)
            } else {
                query.order_by_asc(Column::Description)
            }
        }
        "provider_type" => {
            if sort_desc {
                query.order_by_desc(Column::ProviderType)
            } else {
                query.order_by_asc(Column::ProviderType)
            }
        }
        "model_name" => {
            if sort_desc {
                query.order_by_desc(Column::ModelName)
            } else {
                query.order_by_asc(Column::ModelName)
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

    let items: Vec<LlmAgent> = query
        .offset(offset)
        .limit(limit)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok((items, total))
}

pub async fn find_by_id(id: &str) -> anyhow::Result<Option<LlmAgent>> {
    let model = Entity::find_by_id(id.to_string())
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?;

    Ok(model.map(Into::into))
}

pub async fn find_primary() -> anyhow::Result<Option<LlmAgent>> {
    let model = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::IsPrimary.eq(true))
        .one(conn())
        .await?;

    Ok(model.map(Into::into))
}

pub async fn insert(agent: &LlmAgent) -> anyhow::Result<()> {
    let now = Utc::now();
    let active = ActiveModel {
        id: Set(agent.to_string_id()),
        code: Set(agent.base.code.clone()),
        description: Set(agent.base.description.clone()),
        comment: Set(agent.base.comment.clone()),
        provider_type: Set(agent.provider_type.as_str().to_string()),
        api_endpoint: Set(agent.api_endpoint.clone()),
        api_key: Set(agent.api_key.clone()),
        model_name: Set(agent.model_name.clone()),
        temperature: Set(agent.temperature),
        max_tokens: Set(agent.max_tokens),
        system_prompt: Set(agent.system_prompt.clone()),
        is_primary: Set(agent.is_primary),
        available_models: Set(agent.available_models.clone()),
        is_deleted: Set(false),
        is_posted: Set(false),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        version: Set(1),
    };

    Entity::insert(active).exec(conn()).await?;
    Ok(())
}

pub async fn update(agent: &LlmAgent) -> anyhow::Result<()> {
    let now = Utc::now();
    let active = ActiveModel {
        id: Set(agent.to_string_id()),
        code: Set(agent.base.code.clone()),
        description: Set(agent.base.description.clone()),
        comment: Set(agent.base.comment.clone()),
        provider_type: Set(agent.provider_type.as_str().to_string()),
        api_endpoint: Set(agent.api_endpoint.clone()),
        api_key: Set(agent.api_key.clone()),
        model_name: Set(agent.model_name.clone()),
        temperature: Set(agent.temperature),
        max_tokens: Set(agent.max_tokens),
        system_prompt: Set(agent.system_prompt.clone()),
        is_primary: Set(agent.is_primary),
        available_models: Set(agent.available_models.clone()),
        is_deleted: Set(false),
        is_posted: Set(false),
        created_at: Set(Some(agent.base.metadata.created_at)),
        updated_at: Set(Some(now)),
        version: Set(agent.base.metadata.version + 1),
    };

    Entity::update(active).exec(conn()).await?;
    Ok(())
}

pub async fn soft_delete(id: &str) -> anyhow::Result<()> {
    let now = Utc::now();
    Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Some(now)))
        .filter(Column::Id.eq(id))
        .exec(conn())
        .await?;
    Ok(())
}

/// Снять флаг is_primary со всех агентов
pub async fn clear_all_primary() -> anyhow::Result<()> {
    Entity::update_many()
        .col_expr(Column::IsPrimary, Expr::value(false))
        .exec(conn())
        .await?;
    Ok(())
}
