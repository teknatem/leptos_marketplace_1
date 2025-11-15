use contracts::domain::a004_nomenclature::{ExcelRow, ImportResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::repository;

/// ExcelData для приема с фронтенда (временная структура)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelData {
    pub metadata: ExcelMetadata,
    pub rows: Vec<HashMap<String, String>>,
    pub column_mapping: Vec<ColumnMapping>,
    pub file_headers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelMetadata {
    pub columns: Vec<String>,
    pub row_count: usize,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub expected: String,
    pub found: Option<String>,
    pub file_index: Option<usize>,
}

/// Импортирует данные из ExcelData (принимает весь объект с фронтенда)
/// Конвертирует HashMap в ExcelRow и вызывает основную функцию импорта
pub async fn import_nomenclature_from_excel_data(
    excel_data: ExcelData,
) -> anyhow::Result<ImportResult> {
    // Конвертируем rows (HashMap) в Vec<ExcelRow>
    let rows: Vec<ExcelRow> = excel_data
        .rows
        .into_iter()
        .map(|row_map| ExcelRow {
            article: row_map.get("article").cloned().unwrap_or_default(),
            category: row_map.get("category").cloned().unwrap_or_default(),
            line: row_map.get("line").cloned().unwrap_or_default(),
            model: row_map.get("model").cloned().unwrap_or_default(),
            format: row_map.get("format").cloned().unwrap_or_default(),
            sink: row_map.get("sink").cloned().unwrap_or_default(),
            size: row_map.get("size").cloned().unwrap_or_default(),
        })
        .collect();

    // Вызываем основную функцию импорта
    import_nomenclature_from_rows(rows).await
}

/// Импортирует данные из списка ExcelRow в базу данных
/// Обновляет только те поля, которые НЕ пустые в Excel
/// Если поле в БД заполнено, а в Excel пустое - поле НЕ обновляется
pub async fn import_nomenclature_from_rows(
    rows: Vec<ExcelRow>,
) -> anyhow::Result<ImportResult> {

    let mut updated_count = 0;
    let mut not_found_articles = Vec::new();

    for row in rows {
        // Ищем номенклатуру по артикулу
        let article_trimmed = row.article.trim();
        let found_items = repository::find_by_article(article_trimmed).await?;

        if found_items.is_empty() {
            not_found_articles.push(row.article.clone());
            continue;
        }

        // Обновляем каждую найденную номенклатуру
        for mut item in found_items {
            let mut updated = false;

            // Обновляем только непустые поля из Excel
            // Логика: если в БД заполнено, а в Excel пусто - НЕ обновляем

            if !row.category.is_empty() {
                item.dim1_category = truncate_string(&row.category, 40);
                updated = true;
            }

            if !row.line.is_empty() {
                item.dim2_line = truncate_string(&row.line, 40);
                updated = true;
            }

            if !row.model.is_empty() {
                item.dim3_model = truncate_string(&row.model, 80);
                updated = true;
            }

            if !row.format.is_empty() {
                item.dim4_format = truncate_string(&row.format, 20);
                updated = true;
            }

            if !row.sink.is_empty() {
                item.dim5_sink = truncate_string(&row.sink, 40);
                updated = true;
            }

            if !row.size.is_empty() {
                item.dim6_size = truncate_string(&row.size, 20);
                updated = true;
            }

            if updated {
                item.before_write();
                repository::update(&item).await?;
                updated_count += 1;
            }
        }
    }

    Ok(ImportResult {
        updated_count,
        not_found_articles,
    })
}

/// Обрезает строку до максимальной длины
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 5), "hello");
        assert_eq!(truncate_string("привет мир", 6), "привет");
    }
}
