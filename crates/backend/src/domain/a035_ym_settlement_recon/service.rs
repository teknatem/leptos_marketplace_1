//! Сервис сверки перечислений YM. Документ строится напрямую из p907: строки
//! одного банковского ордера классифицируются в наши обороты (turnover_code),
//! суммируются (теоретическая сумма) и сверяются с фактом YM (bank_sum).

use std::collections::HashMap;

use anyhow::Result;
use sea_orm::{ConnectionTrait, Statement, Value};
use uuid::Uuid;

use contracts::domain::a035_ym_settlement_recon::aggregate::{
    ReconLine, YmSettlementRecon, YmSettlementReconHeader,
};
use contracts::domain::common::AggregateId;

use crate::projections::p907_ym_payment_report::general_ledger_builder::{
    fallback_turnover_code, turnover_code_for_source,
};
use crate::shared::analytics::turnover_registry::get_turnover_class;
use crate::shared::data::db::get_connection;

use super::repository;
pub use repository::{ReconListQuery, ReconListResult, ReconListRow};

fn string_value(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

/// Шапка банковского ордера p907 (для генерации документов).
struct BankOrderHead {
    connection_id: String,
    bank_order_id: i64,
    bank_order_date: String,
    bank_sum: f64,
    period_from: String,
    period_to: String,
}

/// Агрегат строк p907 по одному источнику операции.
struct SourceAgg {
    source: Option<String>,
    sum: f64,
    count: i64,
}

/// Distinct банковские ордера за период (по `bank_order_date`). Пустой период —
/// все ордера. Период покрытых операций — min/max `transaction_date`.
async fn fetch_orders(date_from: &str, date_to: &str) -> Result<Vec<BankOrderHead>> {
    let db = get_connection();
    let mut sql = String::from(
        r#"
        SELECT connection_mp_ref AS connection_id,
               bank_order_id AS bank_order_id,
               substr(MIN(bank_order_date), 1, 10) AS bank_order_date,
               MAX(bank_sum) AS bank_sum,
               substr(MIN(transaction_date), 1, 10) AS period_from,
               substr(MAX(transaction_date), 1, 10) AS period_to
        FROM p907_ym_payment_report
        WHERE bank_order_id IS NOT NULL
          AND bank_order_date IS NOT NULL AND trim(bank_order_date) <> ''
    "#,
    );
    let mut values: Vec<Value> = Vec::new();
    if !date_from.is_empty() {
        sql.push_str(" AND substr(bank_order_date, 1, 10) >= ?");
        values.push(string_value(date_from));
    }
    if !date_to.is_empty() {
        sql.push_str(" AND substr(bank_order_date, 1, 10) <= ?");
        values.push(string_value(date_to));
    }
    sql.push_str(" GROUP BY connection_mp_ref, bank_order_id");

    let stmt = Statement::from_sql_and_values(db.get_database_backend(), &sql, values);
    let rows = db.query_all(stmt).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let connection_id: String = row.try_get("", "connection_id").unwrap_or_default();
        let bank_order_id: i64 = row.try_get("", "bank_order_id").unwrap_or_default();
        let bank_order_date: String = row.try_get("", "bank_order_date").unwrap_or_default();
        let bank_sum: f64 = row.try_get("", "bank_sum").unwrap_or(0.0);
        let period_from: String = row.try_get("", "period_from").unwrap_or_default();
        let period_to: String = row.try_get("", "period_to").unwrap_or_default();
        if connection_id.is_empty() {
            continue;
        }
        out.push(BankOrderHead {
            connection_id,
            bank_order_id,
            bank_order_date,
            bank_sum,
            period_from,
            period_to,
        });
    }
    Ok(out)
}

/// Строки p907 одного ордера, сгруппированные по источнику операции.
async fn fetch_order_sources(connection_id: &str, bank_order_id: i64) -> Result<Vec<SourceAgg>> {
    let db = get_connection();
    let sql = r#"
        SELECT transaction_source AS source,
               SUM(transaction_sum) AS sum,
               COUNT(*) AS cnt
        FROM p907_ym_payment_report
        WHERE connection_mp_ref = ? AND bank_order_id = ?
        GROUP BY transaction_source
    "#;
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        vec![
            string_value(connection_id),
            Value::BigInt(Some(bank_order_id)),
        ],
    );
    let rows = db.query_all(stmt).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(SourceAgg {
            source: row.try_get::<Option<String>>("", "source").unwrap_or(None),
            sum: row.try_get("", "sum").unwrap_or(0.0),
            count: row.try_get("", "cnt").unwrap_or(0),
        });
    }
    Ok(out)
}

/// Классифицировать источники в наши обороты и свернуть по turnover_code.
/// «Платёж покупателя» → customer_revenue, «Оплата услуг» → other_expense и т.п.;
/// неизвестный источник → other_income/other_expense по знаку суммы.
fn build_lines(sources: &[SourceAgg]) -> Vec<ReconLine> {
    // code → (amount, rows_count); порядок фиксируем отдельным вектором ключей.
    let mut acc: HashMap<&'static str, (f64, i64)> = HashMap::new();
    let mut order: Vec<&'static str> = Vec::new();
    for agg in sources {
        let code = match agg.source.as_deref().and_then(turnover_code_for_source) {
            Some(code) => code,
            None => fallback_turnover_code(agg.sum),
        };
        let entry = acc.entry(code).or_insert_with(|| {
            order.push(code);
            (0.0, 0)
        });
        entry.0 += agg.sum;
        entry.1 += agg.count;
    }
    order
        .into_iter()
        .map(|code| {
            let (amount, rows_count) = acc[code];
            let name = get_turnover_class(code)
                .map(|c| c.name.to_string())
                .unwrap_or_else(|| code.to_string());
            ReconLine {
                turnover_code: code.to_string(),
                turnover_name: name,
                amount,
                rows_count: rows_count as i32,
            }
        })
        .collect()
}

/// Разрешить организацию и маркетплейс по подключению (кэшируется вызывающим).
async fn resolve_connection_refs(connection_id: &str) -> (String, String) {
    let Ok(uuid) = Uuid::parse_str(connection_id) else {
        return (String::new(), String::new());
    };
    match crate::domain::a006_connection_mp::service::get_by_id(uuid).await {
        Ok(Some(conn)) => (conn.organization_ref.clone(), conn.marketplace_id.clone()),
        _ => (String::new(), String::new()),
    }
}

/// Построить документ сверки для одного ордера (без сохранения).
async fn build_document(
    head: &BankOrderHead,
    org_marketplace: &(String, String),
) -> Result<YmSettlementRecon> {
    let sources = fetch_order_sources(&head.connection_id, head.bank_order_id).await?;
    let lines = build_lines(&sources);
    let header = YmSettlementReconHeader {
        bank_order_id: head.bank_order_id,
        bank_order_date: head.bank_order_date.clone(),
        connection_id: head.connection_id.clone(),
        organization_id: org_marketplace.0.clone(),
        marketplace_id: org_marketplace.1.clone(),
        period_from: head.period_from.clone(),
        period_to: head.period_to.clone(),
    };
    Ok(YmSettlementRecon::new_for_insert(
        header,
        lines,
        head.bank_sum,
    ))
}

#[derive(Debug, Default)]
pub struct GenerateResult {
    pub created: usize,
    pub updated: usize,
}

/// Найти все банковские ордера за период (пустой — все) и upsert-нуть документы:
/// отсутствующие создать, существующие пересчитать. Один ордер = один документ.
pub async fn generate(date_from: &str, date_to: &str) -> Result<GenerateResult> {
    let orders = fetch_orders(date_from, date_to).await?;
    // Кэш org/marketplace по кабинету — резолвим один раз на кабинет.
    let mut refs_cache: HashMap<String, (String, String)> = HashMap::new();
    let mut result = GenerateResult::default();
    for head in &orders {
        let refs = match refs_cache.get(&head.connection_id) {
            Some(r) => r.clone(),
            None => {
                let r = resolve_connection_refs(&head.connection_id).await;
                refs_cache.insert(head.connection_id.clone(), r.clone());
                r
            }
        };
        let document = build_document(head, &refs).await?;
        if repository::upsert_document(&document).await? {
            result.created += 1;
        } else {
            result.updated += 1;
        }
    }
    Ok(result)
}

/// Пересчитать один существующий документ из текущего p907 (кнопка «Обновить»).
pub async fn recompute(id: Uuid) -> Result<Option<YmSettlementRecon>> {
    let Some(existing) = repository::get_by_id(id).await? else {
        return Ok(None);
    };
    let connection_id = existing.header.connection_id.clone();
    let bank_order_id = existing.header.bank_order_id;

    // Шапка ордера из p907 (bank_sum/даты могли измениться после дозагрузки).
    let orders = fetch_orders("", "").await?;
    let Some(head) = orders
        .into_iter()
        .find(|h| h.connection_id == connection_id && h.bank_order_id == bank_order_id)
    else {
        // Ордер исчез из p907 — оставляем документ как есть.
        return Ok(Some(existing));
    };
    let refs = (
        existing.header.organization_id.clone(),
        existing.header.marketplace_id.clone(),
    );
    let document = build_document(&head, &refs).await?;
    repository::upsert_document(&document).await?;
    repository::get_by_id(Uuid::parse_str(&document.base.id.as_string())?).await
}

pub async fn post_document(id: Uuid) -> Result<()> {
    super::posting::post_document(id).await
}

pub async fn unpost_document(id: Uuid) -> Result<()> {
    super::posting::unpost_document(id).await
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmSettlementRecon>> {
    repository::get_by_id(id).await
}

pub async fn list_paginated(query: ReconListQuery) -> Result<ReconListResult> {
    repository::list_sql(query).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_lines_groups_by_turnover_and_merges_sources() {
        // Платёж + возврат платежа + услуги + неизвестный источник.
        let sources = vec![
            SourceAgg {
                source: Some("Платёж покупателя".to_string()),
                sum: 5_443_314.0,
                count: 476,
            },
            SourceAgg {
                source: Some("Возврат платежа покупателя".to_string()),
                sum: -274_611.0,
                count: 21,
            },
            SourceAgg {
                source: Some("Оплата услуг Яндекс.Маркета".to_string()),
                sum: -813_337.76,
                count: 756,
            },
            SourceAgg {
                source: Some("Премия".to_string()),
                sum: 1000.0,
                count: 1,
            },
        ];
        let lines = build_lines(&sources);

        let revenue = lines
            .iter()
            .find(|l| l.turnover_code == "customer_revenue")
            .expect("customer_revenue line");
        assert_eq!(revenue.amount, 5_443_314.0);
        assert_eq!(revenue.rows_count, 476);

        let storno = lines
            .iter()
            .find(|l| l.turnover_code == "customer_revenue_storno")
            .expect("customer_revenue_storno line");
        assert_eq!(storno.amount, -274_611.0);

        // «Оплата услуг» и «Премия» оба маппятся на other_expense/other_income —
        // разные коды, поэтому отдельные строки.
        let expense = lines
            .iter()
            .find(|l| l.turnover_code == "other_expense")
            .expect("other_expense line");
        assert_eq!(expense.amount, -813_337.76);
        let income = lines
            .iter()
            .find(|l| l.turnover_code == "other_income")
            .expect("other_income line");
        assert_eq!(income.amount, 1000.0);

        // Теоретическая сумма = Σ строк.
        let total: f64 = lines.iter().map(|l| l.amount).sum();
        assert!((total - (5_443_314.0 - 274_611.0 - 813_337.76 + 1000.0)).abs() < 0.001);
    }

    #[test]
    fn build_lines_unknown_source_falls_back_by_sign() {
        let sources = vec![
            SourceAgg {
                source: Some("Неизвестное удержание XYZ".to_string()),
                sum: -500.0,
                count: 2,
            },
            SourceAgg {
                source: None,
                sum: 700.0,
                count: 1,
            },
        ];
        let lines = build_lines(&sources);
        // Отрицательная → other_expense, положительная (в т.ч. NULL-источник) → other_income.
        let expense = lines.iter().find(|l| l.turnover_code == "other_expense");
        let income = lines.iter().find(|l| l.turnover_code == "other_income");
        assert!(expense.is_some());
        assert!(income.is_some());
    }
}
