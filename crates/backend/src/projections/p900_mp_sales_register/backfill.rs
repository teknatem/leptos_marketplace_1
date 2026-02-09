use super::repository::{self, SalesRegisterEntry};
use crate::domain::a007_marketplace_product::service::{
    find_or_create_for_sale, FindOrCreateParams,
};
use anyhow::Result;

/// Статистика backfill операции
#[derive(Debug)]
pub struct BackfillStats {
    pub total_records: usize,
    pub records_updated: usize,
    pub records_skipped: usize,
    pub records_failed: usize,
}

/// Заполнить marketplace_product_ref для всех записей p900, где он NULL
pub async fn backfill_marketplace_product_refs() -> Result<BackfillStats> {
    let mut stats = BackfillStats {
        total_records: 0,
        records_updated: 0,
        records_skipped: 0,
        records_failed: 0,
    };

    tracing::info!("Starting backfill of marketplace_product_ref for p900_sales_register");

    // Получаем все записи с NULL marketplace_product_ref
    let records = repository::get_records_with_null_product_ref().await?;
    stats.total_records = records.len();

    tracing::info!(
        "Found {} records with NULL marketplace_product_ref",
        stats.total_records
    );

    for record in records {
        match backfill_single_record(&record).await {
            Ok(Some(updated_entry)) => {
                // Обновляем запись
                repository::upsert_entry(&updated_entry).await?;
                stats.records_updated += 1;

                if stats.records_updated % 100 == 0 {
                    tracing::info!("Backfilled {} records so far...", stats.records_updated);
                }
            }
            Ok(None) => {
                // Не удалось найти/создать a007
                stats.records_skipped += 1;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to backfill record {}/{}/{}: {}",
                    record.marketplace,
                    record.document_no,
                    record.line_id,
                    e
                );
                stats.records_failed += 1;
            }
        }
    }

    tracing::info!(
        "Backfill completed. Total: {}, Updated: {}, Skipped: {}, Failed: {}",
        stats.total_records,
        stats.records_updated,
        stats.records_skipped,
        stats.records_failed
    );

    Ok(stats)
}

/// Заполнить marketplace_product_ref для одной записи
async fn backfill_single_record(record: &repository::Model) -> Result<Option<SalesRegisterEntry>> {
    // Извлекаем необходимые данные из записи
    let seller_sku = match &record.seller_sku {
        Some(sku) => sku.clone(),
        None => {
            tracing::warn!(
                "Record {}/{}/{} has no seller_sku, skipping",
                record.marketplace,
                record.document_no,
                record.line_id
            );
            return Ok(None);
        }
    };

    let title = record
        .title
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());

    // Определяем marketplace_id по названию маркетплейса
    // ПРИМЕЧАНИЕ: Здесь используется упрощённая логика
    // В реальной системе нужно получать marketplace_id из связанных данных
    let marketplace_id = format!("marketplace-{}-id", record.marketplace.to_lowercase());

    // Поиск или создание a007
    let marketplace_product_ref = match find_or_create_for_sale(FindOrCreateParams {
        marketplace_ref: marketplace_id,
        connection_mp_ref: record.connection_mp_ref.clone(),
        marketplace_sku: seller_sku.clone(),
        barcode: record.barcode.clone(),
        title: title.clone(),
    })
    .await
    {
        Ok(uuid) => uuid,
        Err(e) => {
            tracing::error!(
                "Failed to find/create a007 for record {}/{}/{}: {}",
                record.marketplace,
                record.document_no,
                record.line_id,
                e
            );
            return Err(e);
        }
    };

    // Создаём обновлённую запись
    let updated_entry = SalesRegisterEntry {
        // NK
        marketplace: record.marketplace.clone(),
        document_no: record.document_no.clone(),
        line_id: record.line_id.clone(),

        // Metadata
        scheme: record.scheme.clone(),
        document_type: record.document_type.clone(),
        document_version: record.document_version,

        // References to aggregates
        connection_mp_ref: record.connection_mp_ref.clone(),
        organization_ref: record.organization_ref.clone(),
        marketplace_product_ref: Some(marketplace_product_ref.to_string()), // ОБНОВЛЕНО
        nomenclature_ref: record.nomenclature_ref.clone(), // Сохраняем существующее значение
        registrator_ref: record.registrator_ref.clone(),

        // Timestamps and status
        event_time_source: record
            .event_time_source
            .parse()
            .ok()
            .unwrap_or_else(chrono::Utc::now),
        sale_date: record
            .sale_date
            .parse()
            .ok()
            .unwrap_or_else(|| chrono::Utc::now().date_naive()),
        source_updated_at: record
            .source_updated_at
            .as_ref()
            .and_then(|s| s.parse().ok()),
        status_source: record.status_source.clone(),
        status_norm: record.status_norm.clone(),

        // Product identification
        seller_sku: Some(seller_sku),
        mp_item_id: record.mp_item_id.clone(),
        barcode: record.barcode.clone(),
        title: Some(title),

        // Quantities and money
        qty: record.qty,
        price_list: record.price_list,
        cost: Some(0.00),
        dealer_price_ut: None,
        discount_total: record.discount_total,
        price_effective: record.price_effective,
        amount_line: record.amount_line,
        currency_code: record.currency_code.clone(),
        is_fact: None,

        // Technical
        payload_version: record.payload_version,
        extra: record.extra.clone(),
    };

    Ok(Some(updated_entry))
}
