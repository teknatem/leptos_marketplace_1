use serde::{Deserialize, Serialize};

/// События импорта (для будущего event sourcing)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImportEvent {
    /// Импорт запущен
    ImportStarted {
        session_id: String,
        connection_id: String,
        target_aggregates: Vec<String>,
    },

    /// Импорт завершен
    ImportCompleted {
        session_id: String,
        total_processed: i32,
        total_inserted: i32,
        total_updated: i32,
        total_errors: i32,
    },

    /// Импорт провален
    ImportFailed {
        session_id: String,
        error: String,
    },
}
