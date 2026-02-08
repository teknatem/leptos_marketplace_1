use chrono::Utc;
use contracts::domain::a017_llm_agent::aggregate::LlmAgentId;
use contracts::domain::a018_llm_chat::aggregate::{ArtifactAction, ChatRole, LlmChat, LlmChatAttachment, LlmChatId, LlmChatMessage, LlmChatListItem};
use contracts::domain::a019_llm_artifact::aggregate::LlmArtifactId;
use contracts::domain::common::{AggregateId, BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, FromQueryResult};
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
        pub model_name: String,
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
        pub model_name: Option<String>,
        pub confidence: Option<f64>,
        pub duration_ms: Option<i64>,
        pub created_at: String,
        pub artifact_id: Option<String>,
        pub artifact_action: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

mod attachment {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "a018_llm_chat_attachment")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub message_id: String,
        pub filename: String,
        pub filepath: String,
        pub content_type: String,
        pub file_size: i64,
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
            model_name: m.model_name,
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

        let artifact_id = m.artifact_id.and_then(|id_str| {
            Uuid::parse_str(&id_str).ok().map(LlmArtifactId::new)
        });

        let artifact_action = m.artifact_action.and_then(|action_str| {
            ArtifactAction::from_str(&action_str).ok()
        });

        LlmChatMessage {
            id,
            chat_id: LlmChatId::new(chat_id),
            role,
            content: m.content,
            tokens_used: m.tokens_used,
            model_name: m.model_name,
            confidence: m.confidence,
            duration_ms: m.duration_ms,
            created_at,
            artifact_id,
            artifact_action,
            attachments: Vec::new(), // Загружаются отдельно при необходимости
        }
    }
}

impl From<attachment::Model> for LlmChatAttachment {
    fn from(m: attachment::Model) -> Self {
        let id = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let message_id = Uuid::parse_str(&m.message_id).unwrap_or_else(|_| Uuid::new_v4());
        let created_at = chrono::DateTime::parse_from_rfc3339(&m.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        LlmChatAttachment {
            id,
            message_id,
            filename: m.filename,
            filepath: m.filepath,
            content_type: m.content_type,
            file_size: m.file_size,
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

#[derive(Debug, FromQueryResult)]
struct ChatWithStats {
    id: String,
    code: String,
    description: String,
    agent_id: String,
    agent_name: Option<String>,
    model_name: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    message_count: Option<i64>,
    last_message_at: Option<String>,
}

/// Получить список чатов с подсчетом сообщений и временем последнего сообщения
pub async fn list_with_stats(db: &DatabaseConnection) -> Result<Vec<LlmChatListItem>, DbErr> {
    let sql = r#"
        SELECT 
            c.id,
            c.code,
            c.description,
            c.agent_id,
            a.description as agent_name,
            c.model_name,
            c.created_at,
            COUNT(m.id) as message_count,
            MAX(m.created_at) as last_message_at
        FROM a018_llm_chat c
        LEFT JOIN a017_llm_agent a ON c.agent_id = a.id
        LEFT JOIN a018_llm_chat_message m ON c.id = m.chat_id
        WHERE c.is_deleted = 0
        GROUP BY c.id, c.code, c.description, c.agent_id, a.description, c.model_name, c.created_at
        ORDER BY c.created_at DESC
    "#;

    let results = ChatWithStats::find_by_statement(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        vec![],
    ))
    .all(db)
    .await?;

    Ok(results
        .into_iter()
        .map(|r| {
            let last_message_at = r.last_message_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            LlmChatListItem {
                id: r.id,
                code: r.code,
                description: r.description,
                agent_id: r.agent_id,
                agent_name: r.agent_name,
                model_name: r.model_name,
                created_at: r.created_at.unwrap_or_else(Utc::now),
                message_count: r.message_count,
                last_message_at,
            }
        })
        .collect())
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
        model_name: Set(chat.model_name.clone()),
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
        model_name: Set(chat.model_name.clone()),
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
        model_name: Set(message.model_name.clone()),
        confidence: Set(message.confidence),
        duration_ms: Set(message.duration_ms),
        created_at: Set(message.created_at.to_rfc3339()),
        artifact_id: Set(message.artifact_id.map(|id| id.as_string())),
        artifact_action: Set(message.artifact_action.as_ref().map(|a| a.as_str().to_string())),
    };

    active_model.insert(db).await?;
    Ok(())
}

// ============================================================================
// Attachment Repository Functions
// ============================================================================

/// Найти все вложения для сообщения
pub async fn find_attachments_by_message_id(
    db: &DatabaseConnection,
    message_id: &Uuid,
) -> Result<Vec<LlmChatAttachment>, DbErr> {
    let models = attachment::Entity::find()
        .filter(attachment::Column::MessageId.eq(message_id.to_string()))
        .order_by_asc(attachment::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(models.into_iter().map(|m| m.into()).collect())
}

/// Вставить вложение
pub async fn insert_attachment(db: &DatabaseConnection, attachment: &LlmChatAttachment) -> Result<(), DbErr> {
    let active_model = attachment::ActiveModel {
        id: Set(attachment.id.to_string()),
        message_id: Set(attachment.message_id.to_string()),
        filename: Set(attachment.filename.clone()),
        filepath: Set(attachment.filepath.clone()),
        content_type: Set(attachment.content_type.clone()),
        file_size: Set(attachment.file_size),
        created_at: Set(attachment.created_at.to_rfc3339()),
    };

    active_model.insert(db).await?;
    Ok(())
}

/// Удалить все вложения сообщения
pub async fn delete_attachments_by_message_id(
    db: &DatabaseConnection,
    message_id: &Uuid,
) -> Result<(), DbErr> {
    attachment::Entity::delete_many()
        .filter(attachment::Column::MessageId.eq(message_id.to_string()))
        .exec(db)
        .await?;
    Ok(())
}
