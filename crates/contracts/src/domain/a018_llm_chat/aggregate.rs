use crate::domain::a017_llm_agent::aggregate::LlmAgentId;
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
}

impl LlmChat {
    /// Создать новый чат для вставки в БД
    pub fn new_for_insert(code: String, description: String, agent_id: LlmAgentId) -> Self {
        let base = BaseAggregate::new(LlmChatId::new_v4(), code, description);
        Self { base, agent_id }
    }

    /// Создать чат с известным ID
    pub fn new_with_id(
        id: LlmChatId,
        code: String,
        description: String,
        agent_id: LlmAgentId,
    ) -> Self {
        let base = BaseAggregate::new(id, code, description);
        Self { base, agent_id }
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

/// Сообщение чата
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatMessage {
    pub id: Uuid,
    pub chat_id: LlmChatId,
    pub role: ChatRole,
    pub content: String,
    pub tokens_used: Option<i32>,
    pub created_at: DateTime<Utc>,
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
            created_at: Utc::now(),
        }
    }

    /// Создать сообщение с информацией о токенах
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
            created_at: Utc::now(),
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
            created_at: chat.base.metadata.created_at,
            message_count: None,
            last_message_at: None,
        }
    }
}
