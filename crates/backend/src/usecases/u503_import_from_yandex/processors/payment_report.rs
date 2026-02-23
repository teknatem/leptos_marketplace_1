use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;

use crate::projections::p907_ym_payment_report::repository::{self, YmPaymentReportEntry};

/// Parse and store YM payment report CSV text.
/// Returns (upserted, skipped) counts.
pub async fn process_payment_report_csv(
    connection: &ConnectionMP,
    organization_id: &str,
    csv_text: &str,
) -> Result<(i32, i32)> {
    let connection_mp_ref = connection.base.id.as_string();

    let mut upserted = 0i32;
    let mut skipped = 0i32;

    // Strip UTF-8 BOM if present
    let text = csv_text.trim_start_matches('\u{FEFF}');

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());

    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(e) => {
            anyhow::bail!("Failed to read CSV headers: {}", e);
        }
    };

    tracing::info!(
        "Payment report CSV headers: {:?}",
        headers.iter().collect::<Vec<_>>()
    );

    let mut records_processed = 0usize;

    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Skipping malformed CSV record: {}", e);
                skipped += 1;
                continue;
            }
        };

        // Get field by header name (case-insensitive), returns None if empty
        let get_field = |name: &str| -> Option<String> {
            headers
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name))
                .and_then(|i| record.get(i))
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };

        // Real YM transaction ID — may be empty now, will be filled by YM later.
        // We store it as data but never use it as the key.
        let real_transaction_id = get_field("TRANSACTION_ID");

        // record_key is ALWAYS built from immutable fields so it stays stable
        // across re-imports, including future ones where YM assigns a real TRANSACTION_ID.
        let order_id = get_field("ORDER_ID").unwrap_or_default();
        let date = get_field("TRANSACTION_DATE").unwrap_or_default();
        let typ = get_field("TRANSACTION_TYPE").unwrap_or_default();
        let sum = get_field("TRANSACTION_SUM").unwrap_or_default();

        if order_id.is_empty() && date.is_empty() {
            tracing::warn!("Skipping CSV row: no identifying fields (ORDER_ID and TRANSACTION_DATE both empty)");
            skipped += 1;
            continue;
        }

        let record_key = format!("SYNTH_{}_{}_{}_{}", order_id, date, typ, sum);

        let entry = YmPaymentReportEntry {
            record_key,
            connection_mp_ref: connection_mp_ref.clone(),
            organization_ref: organization_id.to_string(),

            business_id: get_field("BUSINESS_ID").and_then(|v| v.parse::<i64>().ok()),
            partner_id: get_field("PARTNER_ID").and_then(|v| v.parse::<i64>().ok()),
            shop_name: get_field("SHOP_NAME"),
            inn: get_field("INN"),
            model: get_field("MODEL"),

            transaction_id: real_transaction_id, // real YM value (None if empty)
            transaction_date: get_field("TRANSACTION_DATE").map(|d| ru_date_to_iso(&d)),
            transaction_type: get_field("TRANSACTION_TYPE"),
            transaction_source: get_field("TRANSACTION_SOURCE"),
            transaction_sum: get_field("TRANSACTION_SUM").and_then(|v| parse_decimal(&v)),
            payment_status: get_field("PAYMENT_STATUS"),

            order_id: get_field("ORDER_ID").and_then(|v| v.parse::<i64>().ok()),
            shop_order_id: get_field("SHOP_ORDER_ID"),
            order_creation_date: get_field("ORDER_CREATION_DATE").map(|d| ru_date_to_iso(&d)),
            order_delivery_date: get_field("ORDER_DELIVERY_DATE").map(|d| ru_date_to_iso(&d)),
            order_type: get_field("ORDER_TYPE"),

            shop_sku: get_field("SHOP_SKU"),
            offer_or_service_name: get_field("OFFER_OR_SERVICE_NAME"),
            count: get_field("COUNT").and_then(|v| v.parse::<i32>().ok()),

            act_id: get_field("ACT_ID").and_then(|v| v.parse::<i64>().ok()),
            act_date: get_field("ACT_DATE").map(|d| ru_date_to_iso(&d)),
            bank_order_id: get_field("BANK_ORDER_ID").and_then(|v| v.parse::<i64>().ok()),
            bank_order_date: get_field("BANK_ORDER_DATE").map(|d| ru_date_to_iso(&d)),
            bank_sum: get_field("BANK_SUM").and_then(|v| parse_decimal(&v)),

            claim_number: get_field("CLAIM_NUMBER"),
            bonus_account_year_month: get_field("BONUS_ACCOUNT_YEAR_MONTH"),
            comments: get_field("COMMENTS"),

            payload_version: 1,
        };

        match repository::upsert_entry(&entry).await {
            Ok(()) => {
                upserted += 1;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to upsert payment report entry {}: {}",
                    entry.record_key,
                    e
                );
                skipped += 1;
            }
        }

        records_processed += 1;
        if records_processed % 100 == 0 {
            tracing::info!(
                "Payment report progress: {} records processed",
                records_processed
            );
        }
    }

    tracing::info!(
        "Payment report processing complete: {} upserted, {} skipped",
        upserted,
        skipped
    );

    Ok((upserted, skipped))
}

/// Parse decimal number that may use comma as decimal separator (European format)
fn parse_decimal(s: &str) -> Option<f64> {
    let normalized = s.replace(',', ".");
    normalized.parse::<f64>().ok()
}

/// Convert Russian date format DD.MM.YYYY HH:MM to ISO YYYY-MM-DD HH:MM.
/// If format is not recognized, returns the original string unchanged.
fn ru_date_to_iso(s: &str) -> String {
    let bytes = s.as_bytes();
    // Minimum: "DD.MM.YYYY" = 10 chars, dots at positions 2 and 5
    if bytes.len() >= 10 && bytes[2] == b'.' && bytes[5] == b'.' {
        let day = &s[0..2];
        let month = &s[3..5];
        let year = &s[6..10];
        let rest = &s[10..]; // may be " HH:MM" or empty
        return format!("{}-{}-{}{}", year, month, day, rest);
    }
    s.to_string()
}
