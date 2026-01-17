use chrono::Utc;
use contracts::domain::a017_llm_agent::aggregate::LlmAgentId;
use contracts::domain::a018_llm_chat::aggregate::{ChatRole, LlmChat, LlmChatId, LlmChatMessage};
use contracts::domain::common::{AggregateId, BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use sea_orm::prelude::Expr;
use uuid::Uuid;

mod chat {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "a018_llm_chat")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub code: String,
        pub description: String,
        pub comment: Option<String>,
        pub agent_id: String,
        pub is_deleted: bool,
        pub is_posted: bool,
        pub created_at: Option<chrono::DateTime<chrono::Utc>>,
        pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
        pub version: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

mod message {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "a018_llm_chat_message")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub chat_id: String,
        pub role: String,
        pub content: String,
        pub tokens_used: Option<i32>,
        pub created_at: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

impl From<chat::Model> for LlmChat {
    fn from(m: chat::Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let agent_uuid = Uuid::parse_str(&m.agent_id).unwrap_or_else(|_| Uuid::new_v4());

        LlmChat {
            base: BaseAggregate {
                id: LlmChatId::new(uuid),
                code: m.code,
                description: m.description,
                comment: m.comment,
                metadata,
                events: Default::default(),
            },
            agent_id: LlmAgentId::new(agent_uuid),
        }
    }
}

impl From<message::Model> for LlmChatMessage {
    fn from(m: message::Model) -> Self {
        let id = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let chat_id = Uuid::parse_str(&m.chat_id).unwrap_or_else(|_| Uuid::new_v4());
        let role = ChatRole::from_str(&m.role).unwrap_or(ChatRole::User);
        let created_at = chrono::DateTime::parse_from_rfc3339(&m.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        LlmChatMessage {
            id,
            chat_id: LlmChatId::new(chat_id),
            role,
            content: m.content,
            tokens_used: m.tokens_used,
            created_at,
        }
    }
}

// ============================================================================
// Chat Repository Functions
// ============================================================================

/// Получить все чаты (не удаленные)
pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<LlmChat>, DbErr> {
    let models = chat::Entity::find()
        .filter(chat::Column::IsDeleted.eq(false))
        .order_by_desc(chat::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(models.into_iter().map(|m| m.into()).collect())
}

/// Получить чаты с пагинацией
pub async fn list_paginated(
    db: &DatabaseConnection,
    page: u64,
    page_size: u64,
) -> Result<(Vec<LlmChat>, u64), DbErr> {
    let query = chat::Entity::find()
        .filter(chat::Column::IsDeleted.eq(false))
        .order_by_desc(chat::Column::CreatedAt);

    let paginator = query.paginate(db, page_size);
    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page).await?;

    Ok((models.into_iter().map(|m| m.into()).collect(), total))
}

/// Найти чат по ID
pub async fn find_by_id(db: &DatabaseConnection, id: &LlmChatId) -> Result<Option<LlmChat>, DbErr> {
    let model = chat::Entity::find_by_id(id.as_string()).one(db).await?;
    Ok(model.map(|m| m.into()))
}

/// Вставить новый чат
pub async fn insert(db: &DatabaseConnection, chat: &LlmChat) -> Result<(), DbErr> {
    let now = Utc::now();
    let active_model = chat::ActiveModel {
        id: Set(chat.base.id.as_string()),
        code: Set(chat.base.code.clone()),
        description: Set(chat.base.description.clone()),
        comment: Set(chat.base.comment.clone()),
        agent_id: Set(chat.agent_id.as_string()),
        is_deleted: Set(false),
        is_posted: Set(false),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        version: Set(1),
    };

    active_model.insert(db).await?;
    Ok(())
}

/// Обновить чат
pub async fn update(db: &DatabaseConnection, chat: &LlmChat) -> Result<(), DbErr> {
    let now = Utc::now();
    let active_model = chat::ActiveModel {
        id: Set(chat.base.id.as_string()),
        code: Set(chat.base.code.clone()),
        description: Set(chat.base.description.clone()),
        comment: Set(chat.base.comment.clone()),
        agent_id: Set(chat.agent_id.as_string()),
        is_deleted: Set(chat.base.metadata.is_deleted),
        is_posted: Set(chat.base.metadata.is_posted),
        created_at: Set(Some(chat.base.metadata.created_at)),
        updated_at: Set(Some(now)),
        version: Set(chat.base.metadata.version + 1),
    };

    chat::Entity::update(active_model).exec(db).await?;
    Ok(())
}

/// Мягкое удаление чата
pub async fn soft_delete(db: &DatabaseConnection, id: &LlmChatId) -> Result<(), DbErr> {
    let now = Utc::now();
    chat::Entity::update_many()
        .col_expr(chat::Column::IsDeleted, Expr::value(true))
        .col_expr(chat::Column::UpdatedAt, Expr::value(now))
        .filter(chat::Column::Id.eq(id.as_string()))
        .exec(db)
        .await?;
    Ok(())
}

// ============================================================================
// Message Repository Functions
// ============================================================================

/// Найти все сообщения чата
pub async fn find_messages_by_chat_id(
    db: &DatabaseConnection,
    chat_id: &LlmChatId,
) -> Result<Vec<LlmChatMessage>, DbErr> {
    let models = message::Entity::find()
        .filter(message::Column::ChatId.eq(chat_id.as_string()))
        .order_by_asc(message::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(models.into_iter().map(|m| m.into()).collect())
}

/// Вставить сообщение
pub async fn insert_message(db: &DatabaseConnection, message: &LlmChatMessage) -> Result<(), DbErr> {
    let active_model = message::ActiveModel {
        id: Set(message.id.to_string()),
        chat_id: Set(message.chat_id.as_string()),
        role: Set(message.role.as_str().to_string()),
        content: Set(message.content.clone()),
        tokens_used: Set(message.tokens_used),
        created_at: Set(message.created_at.to_rfc3339()),
    };

    active_model.insert(db).await?;
    Ok(())
}
