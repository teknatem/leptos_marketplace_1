use serde::{Deserialize, Serialize};

/// Запись лога системы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: i64,
    pub timestamp: String,
    pub source: String,   // "client" или "server"
    pub category: String,
    pub message: String,
}

/// DTO для создания новой записи лога
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLogRequest {
    pub source: String,
    pub category: String,
    pub message: String,
}
