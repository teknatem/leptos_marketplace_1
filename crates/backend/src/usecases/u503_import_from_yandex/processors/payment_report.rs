use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;

use crate::projections::p907_ym_payment_report::repository::{self, YmPaymentReportEntry};

// ────────────────────────────────────────────────────────────────────────────
// ymid_ key helpers
// ────────────────────────────────────────────────────────────────────────────

/// Map a YM transaction-type string to a single digit code:
///   1 = начисление / выплата / продажа
///   2 = возврат
///   3 = удержание / штраф / сервисный сбор
///   4 = компенсация
///   0 = прочее / неизвестно
pub fn encode_transaction_type(typ: &str) -> u8 {
    let lower = typ.to_lowercase();
    if lower.contains("начисл")
        || lower.contains("выплат")
        || lower.contains("продаж")
        || lower.contains("charge")
    {
        1
    } else if lower.contains("возврат") || lower.contains("return") || lower.contains("refund")
    {
        2
    } else if lower.contains("удержан")
        || lower.contains("штраф")
        || lower.contains("сервис")
        || lower.contains("retention")
        || lower.contains("fee")
    {
        3
    } else if lower.contains("компенсац") || lower.contains("compensation") {
        4
    } else {
        0
    }
}

/// Build the stable `ymid_` record key.
///
/// Format: `"ymid_"` + concatenation of digits only from:
///   order_id · ISO-date digits · type-code (1 digit) · sku-digits · |sum_kopecks|
///
/// All non-digit characters are stripped from the final digit string so the
/// result is always `ymid_` followed by pure digits.
///
/// The function is deterministic for the same logical row across imports:
/// - `order_id`: already numeric, no ambiguity.
/// - date: normalised to ISO before digit-extraction (YYYYMMDDHHII = up to 12 digits).
/// - type_code: always exactly 1 digit.
/// - sku_digits: all digit chars from shop_sku (empty → "" contributes nothing).
/// - sum_kopecks: abs(sum) × 100, rounded to integer.
///
/// Example (from question):
///   order_id=55606680448, date="19.05.2026 00:00", type="Возврат списания",
///   sku="", sum=10009.74
///   → digits: "55606680448" + "202605190000" + "2" + "" + "1000974"
///   → key: "ymid_556066804482026051900002" … + "1000974"
///      = "ymid_55606680448202605190000021000974"
pub fn build_ymid_key(
    order_id: Option<i64>,
    transaction_date: &str,
    transaction_type: &str,
    shop_sku: &str,
    transaction_sum: Option<f64>,
) -> String {
    // order: digits from integer representation
    let order_part = order_id.map(|v| v.to_string()).unwrap_or_default();

    // date: normalise RU → ISO, then keep only digit chars
    let iso = ru_date_to_iso(transaction_date);
    let date_part: String = iso.chars().filter(|c| c.is_ascii_digit()).collect();

    // type: single digit 0–4
    let type_part = encode_transaction_type(transaction_type).to_string();

    // sku: digit chars only (alphanumeric article → keep digits)
    let sku_part: String = shop_sku.chars().filter(|c| c.is_ascii_digit()).collect();

    // sum: |sum| * 100, rounded, as integer string
    let sum_part = transaction_sum
        .map(|f| (f.abs() * 100.0).round() as i64)
        .unwrap_or(0)
        .to_string();

    format!(
        "ymid_{}{}{}{}{}",
        order_part, date_part, type_part, sku_part, sum_part
    )
}

// ────────────────────────────────────────────────────────────────────────────
// CSV processor
// ────────────────────────────────────────────────────────────────────────────

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

        // Real YM transaction ID — may be empty; stored as data but never used as key.
        let real_transaction_id = get_field("TRANSACTION_ID");

        // Fields used for key construction — parsed/raw forms.
        let order_id_num = get_field("ORDER_ID").and_then(|v| v.parse::<i64>().ok());
        let date_raw = get_field("TRANSACTION_DATE").unwrap_or_default();
        let typ_raw = get_field("TRANSACTION_TYPE").unwrap_or_default();
        let sku_raw = get_field("SHOP_SKU").unwrap_or_default();
        let sum_f = get_field("TRANSACTION_SUM").and_then(|v| parse_decimal(&v));

        if order_id_num.is_none() && date_raw.is_empty() {
            tracing::warn!(
                "Skipping CSV row: no identifying fields (ORDER_ID and TRANSACTION_DATE both empty)"
            );
            skipped += 1;
            continue;
        }

        // Stable ymid_ key built from immutable business fields.
        let record_key = build_ymid_key(order_id_num, &date_raw, &typ_raw, &sku_raw, sum_f);

        // Fresh unique id for new records; preserved on upsert conflict via ON CONFLICT exclusion.
        // Standard project format: hyphenated UUID v4 (36 chars), matching all other domain tables.
        let id = uuid::Uuid::new_v4().to_string();

        let entry = YmPaymentReportEntry {
            record_key,
            id,
            connection_mp_ref: connection_mp_ref.clone(),
            organization_ref: organization_id.to_string(),

            business_id: get_field("BUSINESS_ID").and_then(|v| v.parse::<i64>().ok()),
            partner_id: get_field("PARTNER_ID").and_then(|v| v.parse::<i64>().ok()),
            shop_name: get_field("SHOP_NAME"),
            inn: get_field("INN"),
            model: get_field("MODEL"),

            transaction_id: real_transaction_id,
            transaction_date: get_field("TRANSACTION_DATE").map(|d| ru_date_to_iso(&d)),
            transaction_type: get_field("TRANSACTION_TYPE"),
            transaction_source: get_field("TRANSACTION_SOURCE"),
            transaction_sum: sum_f,
            payment_status: get_field("PAYMENT_STATUS"),

            order_id: order_id_num,
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
                crate::projections::p907_ym_payment_report::service::rebuild_record_key_from_existing(
                    &entry.record_key,
                )
                .await?;
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

// ────────────────────────────────────────────────────────────────────────────
// Helpers (also used by repository::migrate_synth_keys via public export)
// ────────────────────────────────────────────────────────────────────────────

/// Parse decimal number that may use comma as decimal separator (European format)
pub fn parse_decimal(s: &str) -> Option<f64> {
    let normalized = s.replace(',', ".");
    normalized.parse::<f64>().ok()
}

/// Convert Russian date format DD.MM.YYYY HH:MM to ISO YYYY-MM-DD HH:MM.
/// If format is not recognised, returns the original string unchanged.
/// Calling on an already-ISO string is safe (idempotent).
pub fn ru_date_to_iso(s: &str) -> String {
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
