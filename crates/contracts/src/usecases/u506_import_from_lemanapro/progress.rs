use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Текущий прогресс импорта (для real-time мониторинга)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub session_id: String,
    pub status: ImportStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Последнее обновление прогресса
    pub updated_at: DateTime<Utc>,

    /// Прогресс по каждому агрегату
    pub aggregates: Vec<AggregateProgress>,

    /// Общая статистика
    pub total_processed: i32,
    pub total_inserted: i32,
    pub total_updated: i32,
    pub total_errors: i32,

    /// Ошибки импорта
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Импорт отменен
    Cancelled,
}

/// Прогресс импорта конкретного агрегата
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateProgress {
    pub aggregate_index: String,
    pub aggregate_name: String,
    pub status: AggregateImportStatus,
    pub processed: i32,
    pub total: Option<i32>,
    pub inserted: i32,
    pub updated: i32,
    pub errors: i32,
    pub current_item: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateImportStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Ошибка импорта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub aggregate_index: Option<String>,
    pub message: String,
    pub details: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl ImportProgress {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            status: ImportStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            updated_at: Utc::now(),
            aggregates: Vec::new(),
            total_processed: 0,
            total_inserted: 0,
            total_updated: 0,
            total_errors: 0,
            errors: Vec::new(),
        }
    }

    pub fn add_error(
        &mut self,
        aggregate_index: Option<String>,
        message: String,
        details: Option<String>,
    ) {
        self.errors.push(ImportError {
            aggregate_index,
            message,
            details,
            occurred_at: Utc::now(),
        });
        self.total_errors += 1;
    }
}
