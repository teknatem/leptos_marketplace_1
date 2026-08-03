use serde::{Deserialize, Serialize};

// ============================================================================
// Перечисления
// ============================================================================

/// Тип тикета (Вопрос / Доработка / Ошибка / Идея)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketType {
    #[serde(rename = "question")]
    Question,
    #[serde(rename = "improvement")]
    Improvement,
    #[serde(rename = "bug")]
    Bug,
    #[serde(rename = "idea")]
    Idea,
}

impl Default for TicketType {
    fn default() -> Self {
        Self::Question
    }
}

impl TicketType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::Improvement => "improvement",
            Self::Bug => "bug",
            Self::Idea => "idea",
        }
    }

    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Question => "Вопрос",
            Self::Improvement => "Доработка",
            Self::Bug => "Ошибка",
            Self::Idea => "Идея",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Question, Self::Improvement, Self::Bug, Self::Idea]
    }
}

impl From<&str> for TicketType {
    fn from(value: &str) -> Self {
        match value {
            "question" => Self::Question,
            "improvement" => Self::Improvement,
            "bug" => Self::Bug,
            "idea" => Self::Idea,
            _ => Self::Question,
        }
    }
}

/// Статус тикета.
///
/// Маппинг на STATUS задачи Битрикс24 (задел под синхронизацию):
/// New→2 (ждёт выполнения), InProgress→3 (выполняется), Waiting→6 (отложена),
/// Resolved→4 (ждёт контроля), Closed→5 (завершена), Rejected→5 (завершена).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketStatus {
    #[serde(rename = "new")]
    New,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "resolved")]
    Resolved,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "rejected")]
    Rejected,
}

impl Default for TicketStatus {
    fn default() -> Self {
        Self::New
    }
}

impl TicketStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::InProgress => "in_progress",
            Self::Waiting => "waiting",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
            Self::Rejected => "rejected",
        }
    }

    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::New => "Новый",
            Self::InProgress => "В работе",
            Self::Waiting => "Ожидание",
            Self::Resolved => "Решён",
            Self::Closed => "Закрыт",
            Self::Rejected => "Отклонён",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::New,
            Self::InProgress,
            Self::Waiting,
            Self::Resolved,
            Self::Closed,
            Self::Rejected,
        ]
    }

    /// STATUS задачи Битрикс24
    pub fn bitrix_status(&self) -> u8 {
        match self {
            Self::New => 2,
            Self::InProgress => 3,
            Self::Waiting => 6,
            Self::Resolved => 4,
            Self::Closed | Self::Rejected => 5,
        }
    }
}

impl From<&str> for TicketStatus {
    fn from(value: &str) -> Self {
        match value {
            "new" => Self::New,
            "in_progress" => Self::InProgress,
            "waiting" => Self::Waiting,
            "resolved" => Self::Resolved,
            "closed" => Self::Closed,
            "rejected" => Self::Rejected,
            _ => Self::New,
        }
    }
}

/// Приоритет тикета (маппится на PRIORITY Битрикс24: 0/1/2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketPriority {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "high")]
    High,
}

impl Default for TicketPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl TicketPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }

    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Low => "Низкий",
            Self::Normal => "Средний",
            Self::High => "Высокий",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Low, Self::Normal, Self::High]
    }

    /// PRIORITY задачи Битрикс24
    pub fn bitrix_priority(&self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
        }
    }
}

impl From<&str> for TicketPriority {
    fn from(value: &str) -> Self {
        match value {
            "low" => Self::Low,
            "normal" => Self::Normal,
            "high" => Self::High,
            _ => Self::Normal,
        }
    }
}

/// Источник тикета
pub const TICKET_ORIGIN_SELF: &str = "self";
pub const TICKET_ORIGIN_LLM_CHAT: &str = "llm_chat";
pub const TICKET_ORIGIN_BITRIX: &str = "bitrix";

// ============================================================================
// DTO
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDto {
    pub id: String,
    pub code: String,
    pub title: String,
    pub description: String,
    pub ticket_type: TicketType,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub deadline: Option<String>,
    pub author_user_id: String,
    pub author_username: Option<String>,
    pub assignee_user_id: Option<String>,
    pub assignee_username: Option<String>,
    pub tags: Vec<String>,
    pub origin: String,
    pub context_page_key: Option<String>,
    pub context_json: Option<String>,
    /// Чат a018, из которого тикет был оформлен (origin = llm_chat). Обратная ссылка
    /// на диалог: по ней разработчик читает исходный разговор с пользователем.
    #[serde(default)]
    pub source_chat_id: Option<String>,
    pub bitrix_task_id: Option<String>,
    #[serde(default)]
    pub bitrix_synced_at: Option<String>,
    #[serde(default)]
    pub bitrix_received_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Количество комментариев (для списка)
    #[serde(default)]
    pub comment_count: i64,
    /// Количество вложений (для списка)
    #[serde(default)]
    pub attachment_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketListResponse {
    pub items: Vec<TicketDto>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTicketRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub ticket_type: TicketType,
    #[serde(default)]
    pub priority: TicketPriority,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub assignee_user_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub context_page_key: Option<String>,
    #[serde(default)]
    pub context_json: Option<String>,
    /// Источник ("self" по умолчанию, "llm_chat" при создании из чата)
    #[serde(default)]
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTicketRequest {
    pub title: String,
    pub description: String,
    pub ticket_type: TicketType,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub assignee_user_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketAttachmentDto {
    pub id: String,
    pub ticket_id: String,
    pub comment_id: Option<String>,
    pub s3_file_id: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub file_size: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketCommentDto {
    pub id: String,
    pub ticket_id: String,
    pub author_user_id: String,
    pub author_username: Option<String>,
    pub body: String,
    pub origin: String,
    #[serde(default)]
    pub bitrix_comment_id: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub attachments: Vec<TicketAttachmentDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTicketCommentRequest {
    pub body: String,
}

/// Полные данные для формы details одним запросом
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDetailsResponse {
    pub ticket: TicketDto,
    pub comments: Vec<TicketCommentDto>,
    /// Все вложения тикета (в т.ч. привязанные к комментариям)
    pub attachments: Vec<TicketAttachmentDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketAttachmentUploadResponse {
    pub attachment: TicketAttachmentDto,
}
