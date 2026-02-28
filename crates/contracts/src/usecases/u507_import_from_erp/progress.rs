use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Текущий прогресс импорта из ERP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub session_id: String,
    pub status: ImportStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,

    /// Прогресс по агрегату a021
    pub processed: i32,
    pub total: Option<i32>,
    pub inserted: i32,
    pub updated: i32,
    pub errors: i32,

    /// Текущий обрабатываемый документ
    pub current_item: Option<String>,

    /// Список ошибок
    pub error_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    /// Импорт запущен
    Running,
    /// Импорт завершен успешно
    Completed,
    /// Импорт завершен с ошибками
    CompletedWithErrors,
    /// Импорт провален
    Failed,
}

impl ImportProgress {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            status: ImportStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            updated_at: Utc::now(),
            processed: 0,
            total: None,
            inserted: 0,
            updated: 0,
            errors: 0,
            current_item: None,
            error_messages: Vec::new(),
        }
    }
}
