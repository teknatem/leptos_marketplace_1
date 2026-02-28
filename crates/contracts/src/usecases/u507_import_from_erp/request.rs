use serde::{Deserialize, Serialize};

/// Запрос на импорт данных из ERP (1С) — Выпуск продукции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// ID подключения к 1С (Connection1CDatabase)
    pub connection_id: String,

    /// Начало периода (включительно), формат YYYY-MM-DD
    pub date_from: String,

    /// Конец периода (включительно), формат YYYY-MM-DD
    pub date_to: String,
}
