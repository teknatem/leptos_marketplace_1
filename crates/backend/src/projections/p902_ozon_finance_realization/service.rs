use anyhow::Result;
use chrono::NaiveDate;

use super::repository::{self, OzonFinanceRealizationEntry};

/// Сохранить строку финансовой реализации
pub async fn upsert_realization_row(entry: OzonFinanceRealizationEntry) -> Result<()> {
    repository::upsert_entry(&entry).await?;
    tracing::debug!(
        "Upserted finance realization: posting={}, sku={}, operation_type={}",
        entry.posting_number,
        entry.sku,
        entry.operation_type
    );
    Ok(())
}

/// Связать записи финансовой реализации с постингом a010 по posting_number
pub async fn link_to_posting(posting_number: &str, posting_uuid: &str) -> Result<usize> {
    let rows = repository::find_by_posting_number(posting_number).await?;

    let mut linked_count = 0;
    for row in rows {
        // Обновляем только если еще не связано
        if row.posting_ref.is_none() {
            repository::update_posting_ref(
                &row.posting_number,
                &row.sku,
                &row.operation_type,
                Some(posting_uuid.to_string()),
            )
            .await?;
            linked_count += 1;
        }
    }

    if linked_count > 0 {
        tracing::info!(
            "Linked {} finance realization rows to posting {} ({})",
            linked_count,
            posting_number,
            posting_uuid
        );
    }

    Ok(linked_count)
}

/// Отвязать записи финансовой реализации от постинга
pub async fn unlink_from_posting(posting_number: &str) -> Result<usize> {
    let rows = repository::find_by_posting_number(posting_number).await?;

    let mut unlinked_count = 0;
    for row in rows {
        if row.posting_ref.is_some() {
            repository::update_posting_ref(&row.posting_number, &row.sku, &row.operation_type, None).await?;
            unlinked_count += 1;
        }
    }

    if unlinked_count > 0 {
        tracing::info!(
            "Unlinked {} finance realization rows from posting {}",
            unlinked_count,
            posting_number
        );
    }

    Ok(unlinked_count)
}

/// Получить статистику по периоду
pub async fn get_stats(
    date_from: NaiveDate,
    date_to: NaiveDate,
    connection_mp_ref: Option<String>,
) -> Result<repository::StatsData> {
    let date_from_str = date_from.format("%Y-%m-%d").to_string();
    let date_to_str = date_to.format("%Y-%m-%d").to_string();

    repository::get_stats(&date_from_str, &date_to_str, connection_mp_ref).await
}
