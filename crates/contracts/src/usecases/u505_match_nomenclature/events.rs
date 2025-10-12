use serde::{Deserialize, Serialize};

/// События процесса сопоставления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchEvent {
    /// Процесс сопоставления запущен
    Started {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "totalItems")]
        total_items: Option<i32>,
    },

    /// Товар обработан
    ItemProcessed {
        article: String,
        #[serde(rename = "productName")]
        product_name: String,
        result: MatchResult,
    },

    /// Процесс завершен
    Completed {
        #[serde(rename = "sessionId")]
        session_id: String,
        matched: i32,
        cleared: i32,
        errors: i32,
    },

    /// Произошла ошибка
    Error {
        message: String,
        details: Option<String>,
    },
}

/// Результат сопоставления одного товара
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchResult {
    /// Успешно сопоставлен (найден 1 элемент номенклатуры)
    Matched {
        #[serde(rename = "nomenclatureId")]
        nomenclature_id: String,
    },

    /// Связь очищена (не найдено совпадений)
    ClearedNotFound,

    /// Связь очищена (найдено >1 совпадений)
    ClearedAmbiguous {
        #[serde(rename = "foundCount")]
        found_count: usize,
    },

    /// Пропущен (уже сопоставлен, overwrite=false)
    Skipped,

    /// Ошибка при обработке
    Error { message: String },
}
