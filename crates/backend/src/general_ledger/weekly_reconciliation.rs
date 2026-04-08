use std::collections::HashMap;

use anyhow::Result;
use contracts::general_ledger::{
    WbWeeklyReconciliationQuery, WbWeeklyReconciliationResponse, WbWeeklyReconciliationRow,
};
use sea_orm::{ConnectionTrait, Statement, Value};

use crate::general_ledger::account_view::registry::ACCOUNT_7609_VIEW;
use crate::shared::data::db::get_connection;

fn conn() -> &'static sea_orm::DatabaseConnection {
    get_connection()
}

fn sv(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn complete_period<'a>(
    from: &'a Option<String>,
    to: &'a Option<String>,
) -> Option<(&'a str, &'a str)> {
    let from = from
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let to = to
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some((from, to))
}

fn overlaps_period(
    row_from: &Option<String>,
    row_to: &Option<String>,
    filter_from: Option<&str>,
    filter_to: Option<&str>,
) -> bool {
    let Some((row_from, row_to)) = complete_period(row_from, row_to) else {
        return true;
    };

    match (filter_from, filter_to) {
        (Some(filter_from), Some(filter_to)) => row_from <= filter_to && row_to >= filter_from,
        (Some(filter_from), None) => row_to >= filter_from,
        (None, Some(filter_to)) => row_from <= filter_to,
        (None, None) => true,
    }
}

fn calculate_difference(
    seller_transfer_total: Option<f64>,
    gl_total_balance: Option<f64>,
) -> Option<f64> {
    Some(seller_transfer_total? - gl_total_balance?)
}

fn build_document_sql(
    query: &WbWeeklyReconciliationQuery,
) -> (String, Vec<Value>, Option<String>, Option<String>) {
    let mut sql = String::from(
        r#"
        SELECT
            d.id AS document_id,
            d.service_name,
            d.connection_id,
            c.description AS connection_name,
            d.report_period_from,
            d.report_period_to,
            CAST(json_extract(d.weekly_report_manual_json, '$.realized_goods_total') AS REAL) AS realized_goods_total,
            CAST(json_extract(d.weekly_report_manual_json, '$.wb_reward_with_vat') AS REAL) AS wb_reward_with_vat,
            CAST(json_extract(d.weekly_report_manual_json, '$.seller_transfer_total') AS REAL) AS seller_transfer_total,
            d.creation_time
        FROM a027_wb_documents d
        LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
        WHERE d.is_deleted = 0
          AND d.is_weekly_report = 1
        "#,
    );

    let mut params = Vec::new();
    let filter_from = normalize_optional(query.date_from.clone());
    let filter_to = normalize_optional(query.date_to.clone());

    if let Some(connection_id) = normalize_optional(query.connection_id.clone()) {
        sql.push_str(" AND d.connection_id = ?");
        params.push(sv(connection_id));
    }

    match (filter_from.as_deref(), filter_to.as_deref()) {
        (Some(filter_from), Some(filter_to)) => {
            sql.push_str(
                " AND ((d.report_period_from IS NOT NULL AND d.report_period_from <> '' AND d.report_period_to IS NOT NULL AND d.report_period_to <> '' AND d.report_period_from <= ? AND d.report_period_to >= ?) OR d.report_period_from IS NULL OR d.report_period_from = '' OR d.report_period_to IS NULL OR d.report_period_to = '')",
            );
            params.push(sv(filter_to));
            params.push(sv(filter_from));
        }
        (Some(filter_from), None) => {
            sql.push_str(
                " AND ((d.report_period_from IS NOT NULL AND d.report_period_from <> '' AND d.report_period_to IS NOT NULL AND d.report_period_to <> '' AND d.report_period_to >= ?) OR d.report_period_from IS NULL OR d.report_period_from = '' OR d.report_period_to IS NULL OR d.report_period_to = '')",
            );
            params.push(sv(filter_from));
        }
        (None, Some(filter_to)) => {
            sql.push_str(
                " AND ((d.report_period_from IS NOT NULL AND d.report_period_from <> '' AND d.report_period_to IS NOT NULL AND d.report_period_to <> '' AND d.report_period_from <= ?) OR d.report_period_from IS NULL OR d.report_period_from = '' OR d.report_period_to IS NULL OR d.report_period_to = '')",
            );
            params.push(sv(filter_to));
        }
        (None, None) => {}
    }

    sql.push_str(
        " ORDER BY CASE WHEN d.report_period_to IS NOT NULL AND d.report_period_to <> '' THEN 0 ELSE 1 END ASC, d.report_period_to DESC, d.creation_time DESC",
    );

    (sql, params, filter_from, filter_to)
}

async fn fetch_gl_total_balance(
    connection_id: &str,
    period_from: &str,
    period_to: &str,
) -> Result<f64> {
    let mut sql = String::from(
        r#"
        SELECT
            COALESCE(SUM(
                CASE
                    WHEN debit_account = '7609' THEN amount
                    WHEN credit_account = '7609' THEN -amount
                    ELSE 0.0
                END
            ), 0.0) AS balance
        FROM sys_general_ledger
        WHERE (debit_account = '7609' OR credit_account = '7609')
          AND entry_date >= ?
          AND entry_date <= ?
          AND connection_mp_ref = ?
          AND (
        "#,
    );

    let mut params = vec![sv(period_from), sv(period_to), sv(connection_id)];

    for (idx, entry) in ACCOUNT_7609_VIEW.main_entries.iter().enumerate() {
        if idx > 0 {
            sql.push_str(" OR ");
        }
        if entry.layer.is_empty() {
            sql.push_str("(turnover_code = ?)");
            params.push(sv(entry.turnover_code));
        } else {
            sql.push_str("(turnover_code = ? AND layer = ?)");
            params.push(sv(entry.turnover_code));
            params.push(sv(entry.layer));
        }
    }

    sql.push_str(")");

    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
    let row = conn().query_one(stmt).await?;

    Ok(row
        .and_then(|raw| raw.try_get::<f64>("", "balance").ok())
        .unwrap_or(0.0))
}

pub async fn get_report(
    query: &WbWeeklyReconciliationQuery,
) -> Result<WbWeeklyReconciliationResponse> {
    let (sql, params, filter_from, filter_to) = build_document_sql(query);
    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
    let rows = conn().query_all(stmt).await?;

    let mut gl_cache: HashMap<(String, String, String), f64> = HashMap::new();
    let mut items = Vec::with_capacity(rows.len());

    for row in rows {
        let document_id: String = row.try_get("", "document_id").unwrap_or_default();
        let service_name: String = row.try_get("", "service_name").unwrap_or_default();
        let connection_id: String = row.try_get("", "connection_id").unwrap_or_default();
        let connection_name: Option<String> = row.try_get("", "connection_name").ok();
        let report_period_from = normalize_optional(row.try_get("", "report_period_from").ok());
        let report_period_to = normalize_optional(row.try_get("", "report_period_to").ok());
        let realized_goods_total = row.try_get("", "realized_goods_total").ok();
        let wb_reward_with_vat = row.try_get("", "wb_reward_with_vat").ok();
        let seller_transfer_total = row.try_get("", "seller_transfer_total").ok();

        if !overlaps_period(
            &report_period_from,
            &report_period_to,
            filter_from.as_deref(),
            filter_to.as_deref(),
        ) {
            continue;
        }

        let gl_total_balance = if let Some((period_from, period_to)) =
            complete_period(&report_period_from, &report_period_to)
        {
            let cache_key = (
                connection_id.clone(),
                period_from.to_string(),
                period_to.to_string(),
            );

            if let Some(balance) = gl_cache.get(&cache_key) {
                Some(*balance)
            } else {
                let balance =
                    fetch_gl_total_balance(&connection_id, period_from, period_to).await?;
                gl_cache.insert(cache_key, balance);
                Some(balance)
            }
        } else {
            None
        };

        items.push(WbWeeklyReconciliationRow {
            document_id,
            service_name,
            connection_id,
            connection_name,
            report_period_from,
            report_period_to,
            realized_goods_total,
            wb_reward_with_vat,
            seller_transfer_total,
            gl_total_balance,
            difference: calculate_difference(seller_transfer_total, gl_total_balance),
        });
    }

    Ok(WbWeeklyReconciliationResponse { items })
}

#[cfg(test)]
pub(crate) fn main_entry_match_expression() -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut params = Vec::new();

    for (idx, entry) in ACCOUNT_7609_VIEW.main_entries.iter().enumerate() {
        if idx > 0 {
            sql.push_str(" OR ");
        }
        if entry.layer.is_empty() {
            sql.push_str("(turnover_code = ?)");
            params.push(sv(entry.turnover_code));
        } else {
            sql.push_str("(turnover_code = ? AND layer = ?)");
            params.push(sv(entry.turnover_code));
            params.push(sv(entry.layer));
        }
    }

    (sql, params)
}

#[cfg(test)]
pub(crate) fn apply_row_values_for_test(
    period_from: Option<&str>,
    period_to: Option<&str>,
    seller_transfer_total: Option<f64>,
    gl_total_balance: Option<f64>,
) -> WbWeeklyReconciliationRow {
    WbWeeklyReconciliationRow {
        document_id: "doc".to_string(),
        service_name: "service".to_string(),
        connection_id: "conn".to_string(),
        connection_name: Some("Cabinet".to_string()),
        report_period_from: period_from.map(|value| value.to_string()),
        report_period_to: period_to.map(|value| value.to_string()),
        realized_goods_total: None,
        wb_reward_with_vat: None,
        seller_transfer_total,
        gl_total_balance,
        difference: calculate_difference(seller_transfer_total, gl_total_balance),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_overlap_accepts_incomplete_document_period() {
        assert!(overlaps_period(
            &None,
            &Some("2026-04-05".to_string()),
            Some("2026-04-01"),
            Some("2026-04-30"),
        ));
    }

    #[test]
    fn period_overlap_checks_intersection_for_complete_period() {
        assert!(overlaps_period(
            &Some("2026-04-01".to_string()),
            &Some("2026-04-07".to_string()),
            Some("2026-04-05"),
            Some("2026-04-30"),
        ));
        assert!(!overlaps_period(
            &Some("2026-03-01".to_string()),
            &Some("2026-03-07".to_string()),
            Some("2026-04-05"),
            Some("2026-04-30"),
        ));
    }

    #[test]
    fn difference_requires_both_values() {
        let row = apply_row_values_for_test(
            Some("2026-04-01"),
            Some("2026-04-07"),
            Some(120.0),
            Some(100.0),
        );
        assert_eq!(row.difference, Some(20.0));

        let row =
            apply_row_values_for_test(Some("2026-04-01"), Some("2026-04-07"), Some(120.0), None);
        assert_eq!(row.difference, None);
    }

    #[test]
    fn main_entry_expression_uses_all_7609_view_entries() {
        let (_, params) = main_entry_match_expression();
        let expected_param_count: usize = ACCOUNT_7609_VIEW
            .main_entries
            .iter()
            .map(|entry| if entry.layer.is_empty() { 1 } else { 2 })
            .sum();

        assert_eq!(params.len(), expected_param_count);
    }
}
