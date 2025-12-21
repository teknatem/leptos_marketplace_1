use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Определение колонки для импорта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    /// Техническое имя поля
    pub field_name: String,
    /// Заголовок для отображения
    pub title: String,
    /// Тип данных
    pub data_type: DataType,
}

/// Типы данных для колонок
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    String,
    Number,
    Date,
}

/// Метаданные Excel файла
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelMetadata {
    /// Список колонок (field_name)
    pub columns: Vec<String>,
    /// Количество строк данных (без заголовка)
    pub row_count: usize,
    /// Имя файла
    pub file_name: String,
}

/// Статус сопоставления колонки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
    /// Ожидаемое имя колонки
    pub expected: String,
    /// Найденное имя колонки в файле (если найдено)
    pub found: Option<String>,
    /// Индекс в файле
    pub file_index: Option<usize>,
}

/// Результат парсинга Excel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelData {
    /// Метаданные
    pub metadata: ExcelMetadata,
    /// Строки данных (HashMap: field_name -> значение)
    pub rows: Vec<HashMap<String, String>>,
    /// Маппинг колонок
    pub column_mapping: Vec<ColumnMapping>,
    /// Заголовки из файла
    pub file_headers: Vec<String>,
}

impl ExcelData {
    /// Создать ExcelData из сырых данных (массив массивов)
    pub fn from_raw(
        raw_data: Vec<Vec<String>>,
        columns: Vec<ColumnDef>,
        file_name: String,
    ) -> Result<Self, String> {
        if raw_data.is_empty() {
            return Err("Файл пуст".to_string());
        }

        // Первая строка - заголовки
        let headers = &raw_data[0];
        let file_headers: Vec<String> = headers.iter().map(|h| h.trim().to_string()).collect();

        // Создаем HashMap для быстрого поиска индексов колонок по заголовкам
        let mut header_indices: HashMap<String, usize> = HashMap::new();
        for (idx, header) in headers.iter().enumerate() {
            header_indices.insert(header.trim().to_lowercase(), idx);
        }

        // Создаем маппинг колонок (проверяем соответствие, но не прерываем)
        let mut column_mapping = Vec::new();
        for col_def in &columns {
            let title_lower = col_def.title.trim().to_lowercase();

            if let Some(&col_idx) = header_indices.get(&title_lower) {
                // Точное совпадение найдено
                column_mapping.push(ColumnMapping {
                    expected: col_def.title.clone(),
                    found: Some(headers[col_idx].trim().to_string()),
                    file_index: Some(col_idx),
                });
            } else {
                // Не найдено - попробуем найти похожее
                let similar = headers.iter().enumerate().find(|(_, h)| {
                    let h_lower = h.trim().to_lowercase();
                    h_lower.contains(&title_lower) || title_lower.contains(&h_lower)
                });

                if let Some((idx, h)) = similar {
                    column_mapping.push(ColumnMapping {
                        expected: col_def.title.clone(),
                        found: Some(h.trim().to_string()),
                        file_index: Some(idx),
                    });
                } else {
                    column_mapping.push(ColumnMapping {
                        expected: col_def.title.clone(),
                        found: None,
                        file_index: None,
                    });
                }
            }
        }

        // Парсим строки данных на основе маппинга
        let mut rows = Vec::new();
        for row in raw_data.iter().skip(1) {
            let mut row_data = HashMap::new();

            for (col_def, mapping) in columns.iter().zip(column_mapping.iter()) {
                if let Some(col_idx) = mapping.file_index {
                    let value = row.get(col_idx).cloned().unwrap_or_default();
                    row_data.insert(col_def.field_name.clone(), value.trim().to_string());
                } else {
                    // Колонка не найдена - пустое значение
                    row_data.insert(col_def.field_name.clone(), String::new());
                }
            }

            // Добавляем только непустые строки (проверяем, что хотя бы одно поле заполнено)
            if row_data.values().any(|v| !v.is_empty()) {
                rows.push(row_data);
            }
        }

        let field_names: Vec<String> = columns.iter().map(|c| c.field_name.clone()).collect();

        Ok(ExcelData {
            metadata: ExcelMetadata {
                columns: field_names,
                row_count: rows.len(),
                file_name,
            },
            rows,
            column_mapping,
            file_headers,
        })
    }

    /// Получить JSON строку для отображения
    pub fn to_json_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Проверить, все ли ожидаемые колонки найдены
    pub fn has_all_columns_mapped(&self) -> bool {
        self.column_mapping.iter().all(|m| m.found.is_some())
    }

    /// Получить количество несопоставленных колонок
    pub fn unmapped_count(&self) -> usize {
        self.column_mapping
            .iter()
            .filter(|m| m.found.is_none())
            .count()
    }
}
