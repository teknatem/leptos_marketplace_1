use serde::{Deserialize, Serialize};

/// Ответ на запуск сопоставления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResponse {
    /// ID сессии сопоставления
    #[serde(rename = "sessionId")]
    pub session_id: String,

    /// Статус запуска
    pub status: MatchStartStatus,

    /// Сообщение
    pub message: String,
}

/// Статус запуска процесса сопоставления
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MatchStartStatus {
    /// Успешно запущен
    Started,
    /// Ошибка при запуске
    Failed,
}
