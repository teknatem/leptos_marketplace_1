//! DTO для пакетов контекста страницы, прикрепляемых к LLM-чату.

use serde::{Deserialize, Serialize};

/// Запрос на добавление контекста текущей страницы к чату.
/// `page_key` — ключ активной вкладки (например `a013_ym_order_details_<id>`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddContextRequest {
    pub page_key: String,
    /// Человекочитаемый заголовок вкладки (фолбэк для title).
    #[serde(default)]
    pub label: Option<String>,
}

/// Краткое представление пакета контекста (для списка/чипа в UI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackageSummary {
    pub id: String,
    pub chat_id: Option<String>,
    pub page_key: String,
    pub page_type: String,
    pub title: String,
    pub created_at: String,
}
