use serde::{Deserialize, Serialize};

/// Строка из Excel файла для импорта номенклатуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelRow {
    pub article: String,
    pub category: String,
    pub line: String,
    pub model: String,
    pub format: String,
    pub sink: String,
    pub size: String,
}

/// Результат импорта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub updated_count: usize,
    pub not_found_articles: Vec<String>,
}
