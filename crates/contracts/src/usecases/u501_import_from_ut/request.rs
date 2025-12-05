use serde::{Deserialize, Serialize};

/// Запрос на импорт данных из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// ID подключения к базе 1С
    pub connection_id: String,

    /// Список агрегатов для импорта (например, ["a002_organization"])
    pub target_aggregates: Vec<String>,

    /// Режим импорта (опционально, для будущего расширения)
    #[serde(default)]
    pub mode: ImportMode,

    /// Удалять записи, которых нет в источнике (жесткое удаление)
    #[serde(default)]
    pub delete_obsolete: bool,

    /// Начало периода для загрузок, зависящих от периода (p906_prices и др.)
    /// Формат: "YYYY-MM-DD"
    #[serde(default)]
    pub period_from: Option<String>,

    /// Конец периода для загрузок, зависящих от периода (p906_prices и др.)
    /// Формат: "YYYY-MM-DD"
    #[serde(default)]
    pub period_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    /// Импорт из UI (интерактивный)
    #[default]
    Interactive,

    /// Фоновый импорт (по расписанию)
    Background,
}
