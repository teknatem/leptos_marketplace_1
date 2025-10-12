use serde::{Deserialize, Serialize};

/// Прогресс сопоставления номенклатуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchProgress {
    /// ID сессии
    #[serde(rename = "sessionId")]
    pub session_id: String,

    /// Статус выполнения
    pub status: MatchStatus,

    /// Всего товаров для обработки
    pub total: Option<i32>,

    /// Обработано товаров
    pub processed: i32,

    /// Сопоставлено (найдено ровно 1 совпадение)
    pub matched: i32,

    /// Очищено (не найдено совпадений или найдено >1)
    pub cleared: i32,

    /// Не изменено (уже были сопоставлены и overwrite_existing=false)
    pub skipped: i32,

    /// Количество товаров с неоднозначным сопоставлением (>1 совпадения)
    pub ambiguous: i32,

    /// Количество ошибок
    pub errors: i32,

    /// Список ошибок
    #[serde(rename = "errorList", default)]
    pub error_list: Vec<MatchError>,

    /// Текущий обрабатываемый товар
    #[serde(rename = "currentItem")]
    pub current_item: Option<String>,

    /// Время начала
    #[serde(rename = "startedAt")]
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Время завершения
    #[serde(rename = "completedAt")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Статус выполнения сопоставления
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MatchStatus {
    /// В процессе
    InProgress,
    /// Завершено успешно
    Completed,
    /// Завершено с ошибками
    CompletedWithErrors,
    /// Провалено
    Failed,
}

/// Информация об ошибке сопоставления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchError {
    /// Описание ошибки
    pub message: String,

    /// Детали ошибки
    pub details: Option<String>,

    /// Артикул товара, при обработке которого произошла ошибка
    pub article: Option<String>,
}
