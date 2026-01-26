use serde::{Deserialize, Serialize};

/// Строка из Excel файла для импорта цен номенклатуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelRow {
    pub date: String,
    pub article: String,
    pub price: String,
}

/// Результат импорта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub updated_count: usize,
    pub not_found_articles: Vec<String>,
}
