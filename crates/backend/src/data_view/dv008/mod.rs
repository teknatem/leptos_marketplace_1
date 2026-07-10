//! dv008 - DataView: WB sales funnel (2 periods)
//!
//! Source: a036_wb_sales_funnel_daily (lines_json exploded via json_each), same
//! data as the "Воронка продаж WB" report plugin (crates/backend/src/plugins/funnel.rs),
//! reimplemented natively here so the LLM chat / BI layer can query it through the
//! ordinary DataView tools instead of invoking the plugin engine.
//!
//! Required params: none (metric defaults to order_sum for scalar).
//! Optional params:
//!   metric = open_count | cart_count | order_count | order_sum | buyout_count |
//!            buyout_sum | cart_conv_pct | order_conv_pct | buyout_pct
//!            (scalar only; default order_sum)

use anyhow::{anyhow, Result};
use contracts::shared::analytics::{IndicatorId, IndicatorStatus, IndicatorValue};
use contracts::shared::data_view::ViewContext;
use contracts::shared::drilldown::{DrilldownResponse, DrilldownRow, MetricColumnDef};
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};
use std::collections::HashMap;

use crate::shared::data::db::get_connection;

const VIEW_ID: &str = "dv008_wb_sales_funnel";
const DEFAULT_METRIC_ID: &str = "order_sum";

const ALL_METRIC_IDS: &[&str] = &[
    "open_count",
    "cart_count",
    "order_count",
    "order_sum",
    "buyout_count",
    "buyout_sum",
    "cart_conv_pct",
    "order_conv_pct",
    "buyout_pct",
];

struct MetricDef {
    /// Aggregate SQL expression over the `d, json_each(d.lines_json) j` row set.
    expr: &'static str,
    label: &'static str,
}

fn resolve_metric_def(id: &str) -> Result<MetricDef> {
    let m = |expr: &'static str, label: &'static str| MetricDef { expr, label };
    match id {
        "open_count" => Ok(m(
            "CAST(COALESCE(SUM(json_extract(j.value, '$.metrics.open_count')), 0) AS REAL)",
            "Просмотры карточки",
        )),
        "cart_count" => Ok(m(
            "CAST(COALESCE(SUM(json_extract(j.value, '$.metrics.cart_count')), 0) AS REAL)",
            "В корзину, шт.",
        )),
        "order_count" => Ok(m(
            "CAST(COALESCE(SUM(json_extract(j.value, '$.metrics.order_count')), 0) AS REAL)",
            "Заказано, шт.",
        )),
        "order_sum" => Ok(m(
            "CAST(COALESCE(SUM(json_extract(j.value, '$.metrics.order_sum')), 0) AS REAL)",
            "Заказано, сумма",
        )),
        "buyout_count" => Ok(m(
            "CAST(COALESCE(SUM(json_extract(j.value, '$.metrics.buyout_count')), 0) AS REAL)",
            "Выкуплено, шт.",
        )),
        "buyout_sum" => Ok(m(
            "CAST(COALESCE(SUM(json_extract(j.value, '$.metrics.buyout_sum')), 0) AS REAL)",
            "Выкуплено, сумма",
        )),
        "cart_conv_pct" => Ok(m(
            "CASE WHEN COALESCE(SUM(json_extract(j.value, '$.metrics.open_count')), 0) = 0 THEN 0 \
             ELSE COALESCE(SUM(json_extract(j.value, '$.metrics.cart_count')), 0) * 100.0 \
                  / SUM(json_extract(j.value, '$.metrics.open_count')) END",
            "Конверсия в корзину, %",
        )),
        "order_conv_pct" => Ok(m(
            "CASE WHEN COALESCE(SUM(json_extract(j.value, '$.metrics.cart_count')), 0) = 0 THEN 0 \
             ELSE COALESCE(SUM(json_extract(j.value, '$.metrics.order_count')), 0) * 100.0 \
                  / SUM(json_extract(j.value, '$.metrics.cart_count')) END",
            "Конверсия в заказ, %",
        )),
        "buyout_pct" => Ok(m(
            "CASE WHEN COALESCE(SUM(json_extract(j.value, '$.metrics.order_count')), 0) = 0 THEN 0 \
             ELSE COALESCE(SUM(json_extract(j.value, '$.metrics.buyout_count')), 0) * 100.0 \
                  / SUM(json_extract(j.value, '$.metrics.order_count')) END",
            "Процент выкупа, %",
        )),
        other => Err(anyhow!(
            "Unsupported metric '{}' for {}. Expected one of: {}",
            other,
            VIEW_ID,
            ALL_METRIC_IDS.join(", ")
        )),
    }
}

fn resolve_metric(ctx: &ViewContext) -> Result<(String, MetricDef)> {
    let id = ctx
        .params
        .get("metric")
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or(DEFAULT_METRIC_ID)
        .to_string();
    let def = resolve_metric_def(&id)?;
    Ok((id, def))
}

fn shift_date(d: &str, months: i32) -> String {
    let parts: Vec<&str> = d.split('-').collect();
    if parts.len() < 3 {
        return d.to_string();
    }
    let y: i32 = parts[0].parse().unwrap_or(2025);
    let m: i32 = parts[1].parse().unwrap_or(1);
    let day: i32 = parts[2].parse().unwrap_or(1);

    let total = y * 12 + (m - 1) + months;
    let ny = total / 12;
    let nm = total % 12 + 1;
    let max_day = match nm {
        2 => {
            if (ny % 4 == 0 && ny % 100 != 0) || ny % 400 == 0 {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    let nd = day.min(max_day);
    format!("{:04}-{:02}-{:02}", ny, nm, nd)
}

fn resolve_period2(ctx: &ViewContext) -> (String, String) {
    match (&ctx.period2_from, &ctx.period2_to) {
        (Some(f), Some(t)) => (f.clone(), t.clone()),
        _ => (shift_date(&ctx.date_from, -1), shift_date(&ctx.date_to, -1)),
    }
}

fn period_label(date_from: &str, date_to: &str) -> String {
    let months = [
        "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
    ];
    let parts: Vec<&str> = date_from.split('-').collect();
    if parts.len() >= 2 {
        let y = parts[0];
        let m: usize = parts[1].parse().unwrap_or(1);
        let month_name = months.get(m.saturating_sub(1)).copied().unwrap_or("?");
        let to_parts: Vec<&str> = date_to.split('-').collect();
        if to_parts.first() == Some(&parts[0]) && to_parts.get(1) == Some(&parts[1]) {
            format!("{} {}", month_name, y)
        } else {
            format!("{} - {}", date_from, date_to)
        }
    } else {
        format!("{} - {}", date_from, date_to)
    }
}

fn pct_change(cur: f64, prev: f64) -> Option<f64> {
    if prev.abs() < 0.01 {
        None
    } else {
        Some(((cur - prev) / prev.abs()) * 100.0)
    }
}

/// Day-of-month label, used so period-1 and period-2 "date" rows line up
/// (01 ↔ 01) even when the two periods fall in different months.
fn day_key(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() >= 3 {
        parts[2].to_string()
    } else {
        iso.to_string()
    }
}

#[derive(Debug, FromQueryResult)]
struct AggRow {
    total: f64,
}

#[derive(Debug, FromQueryResult)]
struct DailyRow {
    total: f64,
}

fn append_connection_filter(sql: &mut String, params: &mut Vec<sea_orm::Value>, refs: &[String]) {
    if refs.is_empty() {
        return;
    }
    let placeholders: Vec<&str> = refs.iter().map(|_| "?").collect();
    sql.push_str(&format!(
        " AND d.connection_id IN ({})",
        placeholders.join(", ")
    ));
    for value in refs {
        params.push(value.clone().into());
    }
}

async fn fetch_aggregate(
    metric_expr: &str,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<f64> {
    let db = get_connection();
    let mut sql = format!(
        r#"
        SELECT {metric_expr} AS total
        FROM a036_wb_sales_funnel_daily d, json_each(d.lines_json) j
        WHERE d.is_deleted = 0 AND d.document_date >= ? AND d.document_date <= ?
        "#
    );
    let mut params: Vec<sea_orm::Value> =
        vec![date_from.to_string().into(), date_to.to_string().into()];
    append_connection_filter(&mut sql, &mut params, connection_mp_refs);

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    let row = AggRow::find_by_statement(stmt)
        .one(db)
        .await?
        .unwrap_or(AggRow { total: 0.0 });
    Ok(row.total)
}

async fn fetch_daily_rows(
    metric_expr: &str,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<Vec<DailyRow>> {
    let db = get_connection();
    let mut sql = format!(
        r#"
        SELECT {metric_expr} AS total
        FROM a036_wb_sales_funnel_daily d, json_each(d.lines_json) j
        WHERE d.is_deleted = 0 AND d.document_date >= ? AND d.document_date <= ?
        "#
    );
    let mut params: Vec<sea_orm::Value> =
        vec![date_from.to_string().into(), date_to.to_string().into()];
    append_connection_filter(&mut sql, &mut params, connection_mp_refs);
    sql.push_str(" GROUP BY d.document_date ORDER BY d.document_date ASC");

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    Ok(DailyRow::find_by_statement(stmt).all(db).await?)
}

fn group_by_label(group_by: &str) -> Result<&'static str> {
    match group_by {
        "nm_id" => Ok("По товару"),
        "date" => Ok("По дню"),
        "connection_mp_ref" => Ok("По кабинету МП"),
        other => Err(anyhow!("Unsupported group_by '{}' for {}", other, VIEW_ID)),
    }
}

/// Fetch one period of drilldown data for all requested metrics in a single query.
async fn fetch_drilldown_multi_period(
    group_by: &str,
    metrics: &[(String, MetricDef)],
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<Vec<(String, String, Vec<f64>)>> {
    let db = get_connection();
    let metric_cols: String = metrics
        .iter()
        .enumerate()
        .map(|(i, (_, def))| format!("{} AS m{}", def.expr, i))
        .collect::<Vec<_>>()
        .join(", ");

    let mut params: Vec<sea_orm::Value> =
        vec![date_from.to_string().into(), date_to.to_string().into()];

    let mut sql = match group_by {
        "date" => format!(
            r#"
            SELECT d.document_date AS group_key, d.document_date AS label, {metric_cols}
            FROM a036_wb_sales_funnel_daily d, json_each(d.lines_json) j
            WHERE d.is_deleted = 0 AND d.document_date >= ? AND d.document_date <= ?
            "#
        ),
        "nm_id" => format!(
            r#"
            SELECT
                CAST(CAST(json_extract(j.value, '$.nm_id') AS INTEGER) AS TEXT) AS group_key,
                COALESCE(MAX(json_extract(j.value, '$.title')), '') AS label,
                {metric_cols}
            FROM a036_wb_sales_funnel_daily d, json_each(d.lines_json) j
            WHERE d.is_deleted = 0 AND d.document_date >= ? AND d.document_date <= ?
            "#
        ),
        "connection_mp_ref" => format!(
            r#"
            SELECT
                d.connection_id AS group_key,
                COALESCE(c.description, d.connection_id) AS label,
                {metric_cols}
            FROM a036_wb_sales_funnel_daily d
            LEFT JOIN a006_connection_mp c ON c.id = d.connection_id, json_each(d.lines_json) j
            WHERE d.is_deleted = 0 AND d.document_date >= ? AND d.document_date <= ?
            "#
        ),
        other => return Err(anyhow!("Unsupported group_by '{}' for {}", other, VIEW_ID)),
    };

    append_connection_filter(&mut sql, &mut params, connection_mp_refs);

    let group_clause = match group_by {
        "date" => " GROUP BY d.document_date ORDER BY group_key ASC",
        "nm_id" => " GROUP BY CAST(json_extract(j.value, '$.nm_id') AS INTEGER) ORDER BY m0 DESC",
        "connection_mp_ref" => " GROUP BY d.connection_id ORDER BY m0 DESC",
        _ => unreachable!(),
    };
    sql.push_str(group_clause);

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    let rows = db.query_all(stmt).await?;
    rows.into_iter()
        .map(|row| {
            let group_key: String = row.try_get("", "group_key")?;
            let label: String = row.try_get("", "label")?;
            let values: Vec<f64> = (0..metrics.len())
                .map(|i| row.try_get::<f64>("", &format!("m{}", i)).unwrap_or(0.0))
                .collect();
            Ok((group_key, label, values))
        })
        .collect::<std::result::Result<Vec<_>, sea_orm::DbErr>>()
        .map_err(anyhow::Error::from)
}

pub async fn compute_scalar(ctx: &ViewContext) -> Result<IndicatorValue> {
    let (metric_id, def) = resolve_metric(ctx)?;
    let (p2_from, p2_to) = resolve_period2(ctx);

    let (current_result, daily_result, prev_result) = tokio::join!(
        fetch_aggregate(
            def.expr,
            &ctx.date_from,
            &ctx.date_to,
            &ctx.connection_mp_refs
        ),
        fetch_daily_rows(
            def.expr,
            &ctx.date_from,
            &ctx.date_to,
            &ctx.connection_mp_refs
        ),
        fetch_aggregate(def.expr, &p2_from, &p2_to, &ctx.connection_mp_refs),
    );

    let current = current_result?;
    let daily = daily_result?;
    let previous = prev_result?;
    let change = pct_change(current, previous);

    Ok(IndicatorValue {
        id: IndicatorId::new(VIEW_ID),
        value: Some(current),
        previous_value: Some(previous),
        change_percent: change,
        status: IndicatorStatus::Neutral,
        subtitle: Some(format!("Воронка продаж WB [{}]", def.label)),
        details: vec![format!("Метрика: {} ({})", def.label, metric_id)],
        spark_points: daily.into_iter().map(|row| row.total).collect(),
    })
}

pub async fn compute_drilldown_multi(
    ctx: &ViewContext,
    group_by: &str,
    metric_ids: &[String],
) -> Result<DrilldownResponse> {
    let group_label = group_by_label(group_by)?;
    let ids: Vec<String> = if metric_ids.is_empty() {
        ALL_METRIC_IDS.iter().map(|s| s.to_string()).collect()
    } else {
        metric_ids.to_vec()
    };
    let metrics: Vec<(String, MetricDef)> = ids
        .iter()
        .map(|id| resolve_metric_def(id).map(|def| (id.clone(), def)))
        .collect::<Result<Vec<_>>>()?;

    let (p2_from, p2_to) = resolve_period2(ctx);
    let is_date_group = group_by == "date";

    let (rows1, rows2) = tokio::join!(
        fetch_drilldown_multi_period(
            group_by,
            &metrics,
            &ctx.date_from,
            &ctx.date_to,
            &ctx.connection_mp_refs
        ),
        fetch_drilldown_multi_period(
            group_by,
            &metrics,
            &p2_from,
            &p2_to,
            &ctx.connection_mp_refs
        ),
    );
    let rows1 = rows1?;
    let rows2 = rows2?;

    let key_of = |raw: &str| {
        if is_date_group {
            day_key(raw)
        } else {
            raw.to_string()
        }
    };

    let mut merged: HashMap<String, DrilldownRow> = HashMap::new();

    for (group_key, label, values) in rows1 {
        let key = key_of(&group_key);
        let label = if is_date_group { key_of(&label) } else { label };
        let entry = merged.entry(key.clone()).or_insert_with(|| DrilldownRow {
            group_key: key,
            label,
            value1: 0.0,
            value2: 0.0,
            delta_pct: None,
            metric_values: HashMap::new(),
        });
        for (i, (id, _)) in metrics.iter().enumerate() {
            let mv = entry.metric_values.entry(id.clone()).or_default();
            mv.value1 += values.get(i).copied().unwrap_or(0.0);
        }
    }

    for (group_key, label, values) in rows2 {
        let key = key_of(&group_key);
        let label = if is_date_group { key_of(&label) } else { label };
        let entry = merged.entry(key.clone()).or_insert_with(|| DrilldownRow {
            group_key: key,
            label,
            value1: 0.0,
            value2: 0.0,
            delta_pct: None,
            metric_values: HashMap::new(),
        });
        for (i, (id, _)) in metrics.iter().enumerate() {
            let mv = entry.metric_values.entry(id.clone()).or_default();
            mv.value2 += values.get(i).copied().unwrap_or(0.0);
        }
    }

    let mut rows: Vec<DrilldownRow> = merged
        .into_values()
        .map(|mut row| {
            for mv in row.metric_values.values_mut() {
                mv.delta_pct = pct_change(mv.value1, mv.value2);
            }
            row
        })
        .collect();

    if is_date_group {
        rows.sort_by(|a, b| a.group_key.cmp(&b.group_key));
    } else {
        let first_id = metrics.first().map(|(id, _)| id.clone());
        rows.sort_by(|a, b| {
            let va = first_id
                .as_ref()
                .and_then(|id| a.metric_values.get(id))
                .map(|mv| mv.value1)
                .unwrap_or(0.0);
            let vb = first_id
                .as_ref()
                .and_then(|id| b.metric_values.get(id))
                .map(|mv| mv.value1)
                .unwrap_or(0.0);
            vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let metric_columns: Vec<MetricColumnDef> = metrics
        .iter()
        .map(|(id, def)| MetricColumnDef {
            id: id.clone(),
            label: def.label.to_string(),
        })
        .collect();

    Ok(DrilldownResponse {
        rows,
        group_by_label: group_label.to_string(),
        period1_label: period_label(&ctx.date_from, &ctx.date_to),
        period2_label: period_label(&p2_from, &p2_to),
        metric_label: String::new(),
        metric_columns,
        selected_dimension: None,
        coverage: None,
        extra_columns: vec![],
        extra_values: HashMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::{day_key, pct_change, resolve_metric_def};

    #[test]
    fn day_key_extracts_day_of_month() {
        assert_eq!(day_key("2026-07-05"), "05");
    }

    #[test]
    fn pct_change_none_for_near_zero_previous() {
        assert_eq!(pct_change(10.0, 0.0), None);
    }

    #[test]
    fn unsupported_metric_is_rejected() {
        assert!(resolve_metric_def("not_a_metric").is_err());
    }

    #[test]
    fn all_known_metrics_resolve() {
        for id in super::ALL_METRIC_IDS {
            assert!(
                resolve_metric_def(id).is_ok(),
                "metric {} should resolve",
                id
            );
        }
    }
}

pub fn meta() -> contracts::shared::data_view::DataViewMeta {
    const JSON: &str = include_str!("metadata.json");
    serde_json::from_str(JSON).expect("dv008/metadata.json parse error")
}
