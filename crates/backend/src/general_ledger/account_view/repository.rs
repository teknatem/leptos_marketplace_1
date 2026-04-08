//! SQL-запрос для GL-ведомости по счёту.

use anyhow::{anyhow, Result};
use sea_orm::{ConnectionTrait, Statement, Value};

use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::shared::data::db::get_connection;
use contracts::general_ledger::{GlAccountViewQuery, GlAccountViewResponse, GlAccountViewRow};

use super::registry::find_view;

fn conn() -> &'static sea_orm::DatabaseConnection {
    get_connection()
}

fn sv(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

/// Строит и выполняет запрос, возвращает заполненный `GlAccountViewResponse`.
pub async fn get_view(query: &GlAccountViewQuery) -> Result<GlAccountViewResponse> {
    let account = query.account.trim();
    if account.is_empty() {
        return Err(anyhow!("account must not be empty"));
    }

    // ── SQL ──────────────────────────────────────────────────────────────────
    //
    // corr_account вычисляется через CASE WHEN:
    //   если debit_account = account → корреспондент = credit_account
    //   иначе                        → корреспондент = debit_account
    //
    // Дт-оборот: SUM(amount) где debit_account = account
    // Кт-оборот: SUM(amount) где credit_account = account
    //
    // GROUP BY turnover_code, layer, corr_account
    let mut sql = String::from(
        r#"
        SELECT
            turnover_code,
            COALESCE(layer, '') AS layer,
            CASE
                WHEN debit_account = ? THEN credit_account
                ELSE debit_account
            END AS corr_account,
            COALESCE(SUM(CASE WHEN debit_account  = ? THEN amount ELSE 0.0 END), 0.0) AS debit_amount,
            COALESCE(SUM(CASE WHEN credit_account = ? THEN amount ELSE 0.0 END), 0.0) AS credit_amount,
            COUNT(*) AS entry_count
        FROM sys_general_ledger
        WHERE (debit_account = ? OR credit_account = ?)
          AND entry_date >= ?
          AND entry_date <= ?
        "#,
    );

    let mut params: Vec<Value> = vec![
        sv(account), // CASE WHEN debit_account = ?
        sv(account), // SUM debit
        sv(account), // SUM credit
        sv(account), // WHERE debit_account = ?
        sv(account), // WHERE credit_account = ?
        sv(query.date_from.clone()),
        sv(query.date_to.clone()),
    ];

    if let Some(cab) = query
        .connection_mp_ref
        .as_ref()
        .filter(|v| !v.trim().is_empty())
    {
        sql.push_str(" AND connection_mp_ref = ?");
        params.push(sv(cab.clone()));
    }
    if let Some(layer) = query.layer.as_ref().filter(|v| !v.trim().is_empty()) {
        sql.push_str(" AND layer = ?");
        params.push(sv(layer.clone()));
    }

    sql.push_str(
        " GROUP BY turnover_code, layer, corr_account ORDER BY turnover_code, layer, corr_account",
    );

    // ── Выполнение ───────────────────────────────────────────────────────────
    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
    let raw_rows = conn().query_all(stmt).await?;

    // ── Разбивка на блоки ────────────────────────────────────────────────────
    let view_def = find_view(account);

    let mut main_rows: Vec<GlAccountViewRow> = Vec::new();
    let mut info_rows: Vec<GlAccountViewRow> = Vec::new();

    for raw in raw_rows {
        let turnover_code: String = raw.try_get("", "turnover_code").unwrap_or_default();
        let layer: String = raw.try_get("", "layer").unwrap_or_default();
        let corr_account: String = raw.try_get("", "corr_account").unwrap_or_default();
        let debit_amount: f64 = raw.try_get("", "debit_amount").unwrap_or(0.0);
        let credit_amount: f64 = raw.try_get("", "credit_amount").unwrap_or(0.0);
        let entry_count: i64 = raw.try_get("", "entry_count").unwrap_or(0);

        let turnover_name = get_turnover_class(&turnover_code)
            .map(|tc| tc.name.to_string())
            .unwrap_or_else(|| turnover_code.clone());

        let row = GlAccountViewRow {
            balance: debit_amount - credit_amount,
            turnover_code: turnover_code.clone(),
            turnover_name,
            corr_account,
            layer: layer.clone(),
            debit_amount,
            credit_amount,
            entry_count,
        };

        let is_main = view_def
            .map(|def| def.is_main_row(&turnover_code, &layer))
            .unwrap_or(false);

        if is_main {
            main_rows.push(row);
        } else {
            info_rows.push(row);
        }
    }

    let total_debit: f64 = main_rows.iter().map(|r| r.debit_amount).sum();
    let total_credit: f64 = main_rows.iter().map(|r| r.credit_amount).sum();
    let total_balance = total_debit - total_credit;

    Ok(GlAccountViewResponse {
        account: account.to_string(),
        main_rows,
        info_rows,
        total_debit,
        total_credit,
        total_balance,
    })
}
