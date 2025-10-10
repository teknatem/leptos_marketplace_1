use serde::{Deserialize, Serialize};

/// События импорта из Yandex Market
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImportEvent {
    /// Импорт запущен
    #[serde(rename = "import_started")]
    ImportStarted {
        session_id: String,
        connection_id: String,
        aggregates: Vec<String>,
    },

    /// Импорт завершен
    #[serde(rename = "import_completed")]
    ImportCompleted {
        session_id: String,
        total_processed: i32,
        total_inserted: i32,
        total_updated: i32,
        total_errors: i32,
    },

    /// Ошибка импорта
    #[serde(rename = "import_failed")]
    ImportFailed {
        session_id: String,
        error: String,
    },
}
