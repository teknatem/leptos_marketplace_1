use contracts::projections::p906_nomenclature_prices::excel::{ExcelRow, ImportResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

use super::repository::{self, NomenclaturePriceEntry};
use crate::domain::a004_nomenclature;
use crate::shared::data::db::get_connection;
use sea_orm::TransactionTrait;

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
pub async fn import_prices_from_excel_data(excel_data: ExcelData) -> anyhow::Result<ImportResult> {
    // Конвертируем rows (HashMap) в Vec<ExcelRow>
    let rows: Vec<ExcelRow> = excel_data
        .rows
        .into_iter()
        .map(|row_map| ExcelRow {
            date: row_map.get("date").cloned().unwrap_or_default(),
            article: row_map.get("article").cloned().unwrap_or_default(),
            price: row_map.get("price").cloned().unwrap_or_default(),
        })
        .collect();

    // Вызываем основную функцию импорта
    import_prices_from_rows(rows).await
}

/// Импортирует данные из списка ExcelRow в базу данных
/// Обновляет/создает записи цен для номенклатур найденных по артикулу
pub async fn import_prices_from_rows(rows: Vec<ExcelRow>) -> anyhow::Result<ImportResult> {
    let started_at = std::time::Instant::now();
    let mut updated_count = 0;
    // Keep unique list (stable order) so UI doesn't show repeated articles.
    let mut not_found_articles: Vec<String> = Vec::new();
    let mut not_found_seen: HashSet<String> = HashSet::new();

    // Single transaction = huge speed-up on SQLite vs 1000 separate commits.
    let db = get_connection();
    let txn = db.begin().await?;

    for (idx, row) in rows.into_iter().enumerate() {
        if idx > 0 && idx % 100 == 0 {
            tracing::info!("Excel import progress: {} rows processed...", idx);
        }

        // Парсим дату (период)
        let period = row.date.trim();
        if period.is_empty() {
            continue; // Пропускаем строки без даты
        }

        // Парсим цену
        let price_str = row.price.trim().replace(",", ".");
        let price: f64 = match price_str.parse() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    "Invalid price '{}' for article '{}'",
                    row.price,
                    row.article
                );
                continue; // Пропускаем строки с невалидной ценой
            }
        };

        // Ищем номенклатуру по артикулу
        let article_trimmed = row.article.trim();
        if article_trimmed.is_empty() {
            continue; // Пропускаем строки без артикула
        }

        let found_items =
            a004_nomenclature::repository::find_by_article_txn(&txn, article_trimmed).await?;

        if found_items.is_empty() {
            let key = article_trimmed.to_string();
            if not_found_seen.insert(key.clone()) {
                not_found_articles.push(key);
            }
            continue;
        }

        // Создаем/обновляем запись цены для каждой найденной номенклатуры
        for item in found_items {
            let nomenclature_ref = item.base.id.value().to_string();

            // Создаем ID для записи (уникальная комбинация period + nomenclature_ref)
            let id = format!("{}_{}", period, &nomenclature_ref);

            let now = chrono::Utc::now();
            let entry = NomenclaturePriceEntry {
                id,
                period: period.to_string(),
                nomenclature_ref,
                price,
                created_at: now,
                updated_at: now,
            };

            repository::upsert_entry_txn(&txn, &entry).await?;
            updated_count += 1;
        }
    }

    txn.commit().await?;

    tracing::info!(
        "Excel import finished: updated_count={}, not_found={}, elapsed_ms={}",
        updated_count,
        not_found_articles.len(),
        started_at.elapsed().as_millis()
    );

    Ok(ImportResult {
        updated_count,
        not_found_articles,
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_price() {
        let price_str = "5309,00".replace(",", ".");
        let price: f64 = price_str.parse().unwrap();
        assert_eq!(price, 5309.00);

        let price_str2 = "3563.00".replace(",", ".");
        let price2: f64 = price_str2.parse().unwrap();
        assert_eq!(price2, 3563.00);
    }
}
