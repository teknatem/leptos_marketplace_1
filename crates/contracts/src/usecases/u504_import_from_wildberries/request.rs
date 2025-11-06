use serde::{Deserialize, Serialize};

/// Запрос на импорт данных из Wildberries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// ID подключения к маркетплейсу (ConnectionMP)
    pub connection_id: String,

    /// Список агрегатов для импорта (например, ["a007_marketplace_product", "a012_wb_sales"])
    pub target_aggregates: Vec<String>,

    /// Дата начала периода (для импорта продаж)
    #[serde(default = "default_date_from")]
    pub date_from: chrono::NaiveDate,

    /// Дата окончания периода (для импорта продаж)
    #[serde(default = "default_date_to")]
    pub date_to: chrono::NaiveDate,

    /// Режим импорта (опционально, для будущего расширения)
    #[serde(default)]
    pub mode: ImportMode,
}

fn default_date_from() -> chrono::NaiveDate {
    chrono::Utc::now().naive_utc().date() - chrono::Duration::days(30)
}

fn default_date_to() -> chrono::NaiveDate {
    chrono::Utc::now().naive_utc().date()
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
