use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Ошибки LLM провайдера
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Provider not supported: {0}")]
    UnsupportedProvider(String),
}

/// Роль сообщения в чате
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// Сообщение чата
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
        }
    }
}

/// Ответ от LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tokens_used: Option<i32>,
    pub model: String,
    pub finish_reason: Option<String>,
}

/// Трейт для LLM провайдеров
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Отправка запроса к чату
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<LlmResponse, LlmError>;

    /// Тест подключения к провайдеру
    async fn test_connection(&self) -> Result<(), LlmError>;

    /// Получить название провайдера
    fn provider_name(&self) -> &str;
}
