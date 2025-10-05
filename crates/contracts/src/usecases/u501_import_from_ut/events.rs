use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// События импорта для event sourcing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImportEvent {
    /// Импорт запущен
    Started {
        session_id: String,
        connection_id: String,
        target_aggregates: Vec<String>,
        started_at: DateTime<Utc>,
    },

    /// Прогресс импорта агрегата
    Progress {
        session_id: String,
        aggregate_index: String,
        aggregate_name: String,
        processed: i32,
        total: Option<i32>,
        inserted: i32,
        updated: i32,
    },

    /// Ошибка при импорте
    Error {
        session_id: String,
        aggregate_index: Option<String>,
        error_message: String,
        details: Option<String>,
    },

    /// Импорт завершен
    Completed {
        session_id: String,
        completed_at: DateTime<Utc>,
        total_processed: i32,
        total_inserted: i32,
        total_updated: i32,
        total_errors: i32,
        duration_ms: u64,
    },

    /// Импорт отменен
    Cancelled {
        session_id: String,
        cancelled_at: DateTime<Utc>,
        reason: String,
    },
}

impl ImportEvent {
    pub fn session_id(&self) -> &str {
        match self {
            ImportEvent::Started { session_id, .. } => session_id,
            ImportEvent::Progress { session_id, .. } => session_id,
            ImportEvent::Error { session_id, .. } => session_id,
            ImportEvent::Completed { session_id, .. } => session_id,
            ImportEvent::Cancelled { session_id, .. } => session_id,
        }
    }
}
