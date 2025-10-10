use serde::{Deserialize, Serialize};

/// Ответ на запрос импорта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResponse {
    /// Уникальный ID сессии импорта
    pub session_id: String,

    /// Статус запуска
    pub status: ImportStartStatus,

    /// Сообщение
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportStartStatus {
    /// Импорт успешно запущен
    Started,

    /// Ошибка при запуске
    Failed,
}
