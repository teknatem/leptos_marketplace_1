use serde::{Deserialize, Serialize};

/// Запрос на импорт данных из OZON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// ID подключения к маркетплейсу (ConnectionMP)
    pub connection_id: String,

    /// Список агрегатов для импорта (например, ["a007_marketplace_product"])
    pub target_aggregates: Vec<String>,

    /// Режим импорта (опционально, для будущего расширения)
    #[serde(default)]
    pub mode: ImportMode,
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
