use crate::domain::a017_llm_agent::aggregate::LlmAgentId;
use crate::domain::a019_llm_artifact::aggregate::LlmArtifactId;
use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID типа для агрегата LLM Chat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmChatId(pub Uuid);

impl LlmChatId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl AggregateId for LlmChatId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(LlmChatId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

/// Роль сообщения в чате
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

impl ChatRole {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "system" => Ok(ChatRole::System),
            "user" => Ok(ChatRole::User),
            "assistant" => Ok(ChatRole::Assistant),
            _ => Err(format!("Unknown chat role: {}", s)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        }
    }
}

/// Агрегат LLM Chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChat {
    #[serde(flatten)]
    pub base: BaseAggregate<LlmChatId>,
    pub agent_id: LlmAgentId,
    pub model_name: String,
}

impl LlmChat {
    /// Создать новый чат для вставки в БД
    pub fn new_for_insert(
        code: String,
        description: String,
        agent_id: LlmAgentId,
        model_name: String,
    ) -> Self {
        let base = BaseAggregate::new(LlmChatId::new_v4(), code, description);
        Self {
            base,
            agent_id,
            model_name,
        }
    }

    /// Создать чат с известным ID
    pub fn new_with_id(
        id: LlmChatId,
        code: String,
        description: String,
        agent_id: LlmAgentId,
        model_name: String,
    ) -> Self {
        let base = BaseAggregate::new(id, code, description);
        Self {
            base,
            agent_id,
            model_name,
        }
    }

    pub fn to_string_id(&self) -> String {
        self.base.id.as_string()
    }

    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.base.description.trim().is_empty() {
            return Err("Описание не может быть пустым".into());
        }
        if self.base.code.trim().is_empty() {
            return Err("Код не может быть пустым".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }
}

/// Чат с подставленным именем агента (для детальной страницы / API get by id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatDetail {
    #[serde(flatten)]
    pub chat: LlmChat,
    /// Имя агента (description из a017_llm_agent), если известно.
    pub agent_name: Option<String>,
}

impl AggregateRoot for LlmChat {
    type Id = LlmChatId;

    fn id(&self) -> Self::Id {
        self.base.id
    }

    fn code(&self) -> &str {
        &self.base.code
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn metadata(&self) -> &EntityMetadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut EntityMetadata {
        &mut self.base.metadata
    }

    fn events(&self) -> &EventStore {
        &self.base.events
    }

    fn events_mut(&mut self) -> &mut EventStore {
        &mut self.base.events
    }

    fn aggregate_index() -> &'static str {
        "a018"
    }

    fn collection_name() -> &'static str {
        "llm_chat"
    }

    fn element_name() -> &'static str {
        "Чат LLM"
    }

    fn list_name() -> &'static str {
        "Чаты LLM"
    }

    fn origin() -> Origin {
        Origin::Self_
    }
}

/// Вложение к сообщению чата
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatAttachment {
    pub id: Uuid,
    pub message_id: Uuid,
    pub filename: String,
    pub filepath: String,
    pub content_type: String,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

impl LlmChatAttachment {
    pub fn new(
        message_id: Uuid,
        filename: String,
        filepath: String,
        content_type: String,
        file_size: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_id,
            filename,
            filepath,
            content_type,
            file_size,
            created_at: Utc::now(),
        }
    }
}

/// Действие с артефактом
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactAction {
    Created,
    Updated,
}

impl ArtifactAction {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "created" => Ok(ArtifactAction::Created),
            "updated" => Ok(ArtifactAction::Updated),
            _ => Err(format!("Unknown artifact action: {}", s)),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ArtifactAction::Created => "created",
            ArtifactAction::Updated => "updated",
        }
    }
}

/// Сообщение чата
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatMessage {
    pub id: Uuid,
    pub chat_id: LlmChatId,
    pub role: ChatRole,
    pub content: String,
    pub tokens_used: Option<i32>,
    pub model_name: Option<String>,
    pub confidence: Option<f64>,
    pub duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    
    // Связь с артефактами
    pub artifact_id: Option<LlmArtifactId>,
    pub artifact_action: Option<ArtifactAction>,
    
    // Вложения (загружаются отдельно при необходимости)
    #[serde(default)]
    pub attachments: Vec<LlmChatAttachment>,
}

impl LlmChatMessage {
    /// Создать новое сообщение
    pub fn new(chat_id: LlmChatId, role: ChatRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            chat_id,
            role,
            content,
            tokens_used: None,
            model_name: None,
            confidence: None,
            duration_ms: None,
            created_at: Utc::now(),
            artifact_id: None,
            artifact_action: None,
            attachments: Vec::new(),
        }
    }

    /// Создать сообщение с полной информацией
    pub fn new_with_metadata(
        chat_id: LlmChatId,
        role: ChatRole,
        content: String,
        tokens_used: Option<i32>,
        model_name: Option<String>,
        confidence: Option<f64>,
        duration_ms: Option<i64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            chat_id,
            role,
            content,
            tokens_used,
            model_name,
            confidence,
            duration_ms,
            created_at: Utc::now(),
            artifact_id: None,
            artifact_action: None,
            attachments: Vec::new(),
        }
    }

    /// Создать сообщение с информацией о токенах (deprecated, использовать new_with_metadata)
    pub fn new_with_tokens(
        chat_id: LlmChatId,
        role: ChatRole,
        content: String,
        tokens_used: Option<i32>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            chat_id,
            role,
            content,
            tokens_used,
            model_name: None,
            confidence: None,
            duration_ms: None,
            created_at: Utc::now(),
            artifact_id: None,
            artifact_action: None,
            attachments: Vec::new(),
        }
    }

    /// Создать системное сообщение
    pub fn system(chat_id: LlmChatId, content: impl Into<String>) -> Self {
        Self::new(chat_id, ChatRole::System, content.into())
    }

    /// Создать сообщение пользователя
    pub fn user(chat_id: LlmChatId, content: impl Into<String>) -> Self {
        Self::new(chat_id, ChatRole::User, content.into())
    }

    /// Создать сообщение ассистента
    pub fn assistant(chat_id: LlmChatId, content: impl Into<String>) -> Self {
        Self::new(chat_id, ChatRole::Assistant, content.into())
    }
}

/// DTO для элемента списка чатов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatListItem {
    pub id: String,
    pub code: String,
    pub description: String,
    pub agent_id: String,
    pub agent_name: Option<String>,
    pub model_name: String,
    pub created_at: DateTime<Utc>,
    pub message_count: Option<i64>,
    pub last_message_at: Option<DateTime<Utc>>,
}

impl From<LlmChat> for LlmChatListItem {
    fn from(chat: LlmChat) -> Self {
        Self {
            id: chat.base.id.as_string(),
            code: chat.base.code,
            description: chat.base.description,
            agent_id: chat.agent_id.as_string(),
            agent_name: None,
            model_name: chat.model_name,
            created_at: chat.base.metadata.created_at,
            message_count: None,
            last_message_at: None,
        }
    }
}
