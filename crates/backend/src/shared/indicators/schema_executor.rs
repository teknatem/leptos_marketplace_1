//! Schema-based indicator executor
//!
//! Выполняет вычисление значений индикаторов и drilldown-запросы
//! на основе конфигурации схемы (P904SchemaConfig).

use anyhow::Result;
use contracts::domain::a024_bi_indicator::aggregate::{
    IndicatorDataSourceConfig, P904MetricColumn, P904SchemaConfig,
};
use contracts::shared::drilldown::{DrilldownRequest, DrilldownResponse, DrilldownRow};
use contracts::shared::indicators::{IndicatorContext, IndicatorId, IndicatorStatus, IndicatorValue};
use sea_orm::{FromQueryResult, Statement};

use crate::shared::data::db::get_connection;
use crate::shared::universal_dashboard::get_registry;
#[allow(unused_imports)]
use contracts::shared::universal_dashboard::DataSourceSchemaOwned;

// ---------------------------------------------------------------------------
// Period helpers
// ---------------------------------------------------------------------------

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

fn pct_change(cur: f64, prev: f64) -> Option<f64> {
    if prev.abs() < 0.01 {
        None
    } else {
        Some(((cur - prev) / prev.abs()) * 100.0)
    }
}

fn status_by_change(change: Option<f64>, higher_is_good: bool) -> IndicatorStatus {
    match change {
        Some(c) if c > 5.0 => {
            if higher_is_good {
                IndicatorStatus::Good
            } else {
                IndicatorStatus::Bad
            }
        }
        Some(c) if c < -5.0 => {
            if higher_is_good {
                IndicatorStatus::Bad
            } else {
                IndicatorStatus::Good
            }
        }
        _ => IndicatorStatus::Neutral,
    }
}

fn period_label(date_from: &str, date_to: &str) -> String {
    // Abbreviated label: "янв 2026" style from date_from
    let months = [
        "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
    ];
    let parts: Vec<&str> = date_from.split('-').collect();
    if parts.len() >= 2 {
        let y = parts[0];
        let m: usize = parts[1].parse().unwrap_or(1);
        let month_name = months.get(m.saturating_sub(1)).copied().unwrap_or("?");
        // If from and to are in same month, just show month+year
        let to_parts: Vec<&str> = date_to.split('-').collect();
        if to_parts.get(1) == Some(&parts[1]) && to_parts.get(0) == Some(&parts[0]) {
            format!("{} {}", month_name, y)
        } else {
            format!("{} – {}", date_from, date_to)
        }
    } else {
        format!("{} – {}", date_from, date_to)
    }
}

// ---------------------------------------------------------------------------
// P904 aggregation row
// ---------------------------------------------------------------------------

#[derive(Debug, FromQueryResult)]
struct P904Agg {
    total: f64,
}

/// Build the metric expression for aggregation
fn metric_expr(metric: &P904MetricColumn) -> &'static str {
    match metric {
        P904MetricColumn::CustomerIn => "CAST(COALESCE(SUM(p.customer_in), 0) AS REAL)",
        P904MetricColumn::CustomerOut => "CAST(COALESCE(SUM(p.customer_out), 0) AS REAL)",
        P904MetricColumn::SellerOut => "CAST(COALESCE(SUM(p.seller_out), 0) AS REAL)",
        P904MetricColumn::OrderCount => {
            "CAST(COUNT(DISTINCT p.registrator_ref) AS REAL)"
        }
    }
}

async fn fetch_p904_agg(
    metric: &P904MetricColumn,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<f64> {
    let db = get_connection();

    let mut sql = format!(
        r#"
        SELECT {metric_expr} AS total
        FROM p904_sales_data p
        WHERE p.date >= ? AND p.date <= ?
        "#,
        metric_expr = metric_expr(metric)
    );

    let mut params: Vec<sea_orm::Value> =
        vec![date_from.to_string().into(), date_to.to_string().into()];

    if !connection_mp_refs.is_empty() {
        let placeholders: Vec<&str> = connection_mp_refs.iter().map(|_| "?").collect();
        sql.push_str(&format!(
            " AND p.connection_mp_ref IN ({})",
            placeholders.join(", ")
        ));
        for r in connection_mp_refs {
            params.push(r.clone().into());
        }
    }

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    let row = P904Agg::find_by_statement(stmt)
        .one(db)
        .await?
        .unwrap_or(P904Agg { total: 0.0 });

    Ok(row.total)
}

// ---------------------------------------------------------------------------
// Public: compute indicator value based on P904SchemaConfig
// ---------------------------------------------------------------------------

pub async fn compute_p904(
    config: &P904SchemaConfig,
    ctx: &IndicatorContext,
    indicator_id: IndicatorId,
) -> Result<IndicatorValue> {
    let cur = fetch_p904_agg(
        &config.metric,
        &ctx.date_from,
        &ctx.date_to,
        &ctx.connection_mp_refs,
    )
    .await?;

    // Period 2: use extra["period2_from"] / extra["period2_to"] or auto-shift 1 month back
    let (p2_from, p2_to) = if let (Some(f), Some(t)) =
        (ctx.extra.get("period2_from"), ctx.extra.get("period2_to"))
    {
        (f.clone(), t.clone())
    } else {
        (
            shift_date(&ctx.date_from, -1),
            shift_date(&ctx.date_to, -1),
        )
    };

    let prev = fetch_p904_agg(
        &config.metric,
        &p2_from,
        &p2_to,
        &ctx.connection_mp_refs,
    )
    .await?;

    let change = pct_change(cur, prev);
    let higher_is_good = !matches!(config.metric, P904MetricColumn::CustomerOut);

    Ok(IndicatorValue {
        id: indicator_id,
        value: Some(cur),
        previous_value: Some(prev),
        change_percent: change,
        status: status_by_change(change, higher_is_good),
        subtitle: None,
        spark_points: vec![],
    })
}

// ---------------------------------------------------------------------------
// Generic compute from IndicatorDataSourceConfig
// ---------------------------------------------------------------------------

/// Generic single-value aggregation using schema registry.
async fn fetch_schema_agg(
    table_name: &str,
    date_col: &str,
    metric_expr: &str,
    connection_col: Option<&str>,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<f64> {
    let db = get_connection();

    let mut sql = format!(
        "SELECT {metric_expr} AS total FROM {table_name} p \
         WHERE p.{date_col} >= ? AND p.{date_col} <= ?",
    );

    let mut params: Vec<sea_orm::Value> =
        vec![date_from.to_string().into(), date_to.to_string().into()];

    if !connection_mp_refs.is_empty() {
        if let Some(col) = connection_col {
            let placeholders: Vec<&str> = connection_mp_refs.iter().map(|_| "?").collect();
            sql.push_str(&format!(
                " AND p.{col} IN ({})",
                placeholders.join(", ")
            ));
            for r in connection_mp_refs {
                params.push(r.clone().into());
            }
        }
    }

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    let row = P904Agg::find_by_statement(stmt)
        .one(db)
        .await?
        .unwrap_or(P904Agg { total: 0.0 });

    Ok(row.total)
}

/// Compute indicator value using the universal `IndicatorDataSourceConfig`.
///
/// Resolves the schema from SchemaRegistry, finds the metric field, and
/// runs aggregation for the current period + auto-shifted previous period.
pub async fn compute_from_data_source(
    config: &IndicatorDataSourceConfig,
    ctx: &IndicatorContext,
    indicator_id: IndicatorId,
) -> Result<IndicatorValue> {
    let registry = get_registry();
    let table_name = registry
        .get_table_name(&config.schema_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown schema: {}", config.schema_id))?;

    let schema = registry
        .get_schema(&config.schema_id)
        .ok_or_else(|| anyhow::anyhow!("Schema not found: {}", config.schema_id))?;

    // Find the metric field
    let metric_field = schema
        .fields
        .iter()
        .find(|f| f.id == config.metric_field_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Metric field '{}' not found in schema '{}'",
                config.metric_field_id,
                config.schema_id
            )
        })?
        .clone();

    // Build the SQL metric expression.
    // Heuristic: Integer fields that represent counts use COUNT DISTINCT on their db_column.
    // All numeric/aggregate fields use SUM.
    let metric_expr = build_metric_expr(&metric_field.db_column, &metric_field.id);

    // Find the date column (first Date/DateTime field in schema)
    let date_col = schema
        .fields
        .iter()
        .find(|f| {
            matches!(
                f.value_type,
                contracts::shared::universal_dashboard::ValueType::Date
                    | contracts::shared::universal_dashboard::ValueType::DateTime
            )
        })
        .map(|f| f.db_column.clone())
        .unwrap_or_else(|| "date".to_string());

    // Find connection_mp_ref column if the schema has it and it's in context_filter_fields
    let connection_col = if config
        .context_filter_fields
        .iter()
        .any(|f| f == "connection_mp_ref")
    {
        schema
            .fields
            .iter()
            .find(|f| f.id == "connection_mp_ref")
            .map(|f| f.db_column.clone())
    } else {
        None
    };

    let cur = fetch_schema_agg(
        &table_name,
        &date_col,
        &metric_expr,
        connection_col.as_deref(),
        &ctx.date_from,
        &ctx.date_to,
        &ctx.connection_mp_refs,
    )
    .await?;

    // Period 2: use extra["period2_from"] / extra["period2_to"] or auto-shift 1 month back
    let (p2_from, p2_to) = if let (Some(f), Some(t)) =
        (ctx.extra.get("period2_from"), ctx.extra.get("period2_to"))
    {
        (f.clone(), t.clone())
    } else {
        (
            shift_date(&ctx.date_from, -1),
            shift_date(&ctx.date_to, -1),
        )
    };

    let prev = fetch_schema_agg(
        &table_name,
        &date_col,
        &metric_expr,
        connection_col.as_deref(),
        &p2_from,
        &p2_to,
        &ctx.connection_mp_refs,
    )
    .await?;

    let change = pct_change(cur, prev);
    // "Higher is good" for all metrics except those with "return" / "out" semantics that mean cost.
    // By convention: if metric field id contains "out" but not "seller_out", it's a cost → lower is good.
    // Simple heuristic: `customer_out` (returns paid to customer) → lower is good.
    let higher_is_good = !metric_field.id.contains("customer_out");

    Ok(IndicatorValue {
        id: indicator_id,
        value: Some(cur),
        previous_value: Some(prev),
        change_percent: change,
        status: status_by_change(change, higher_is_good),
        subtitle: None,
        spark_points: vec![],
    })
}

/// Build a SQL metric expression for a field.
///
/// Fields whose ID ends in "_count" or equals "order_count" are treated as
/// COUNT DISTINCT; all others are SUM.
fn build_metric_expr(db_column: &str, field_id: &str) -> String {
    if field_id == "order_count" || field_id.ends_with("_count") {
        format!("CAST(COUNT(DISTINCT p.{db_column}) AS REAL)")
    } else {
        format!("CAST(COALESCE(SUM(p.{db_column}), 0) AS REAL)")
    }
}

// ---------------------------------------------------------------------------
// Universal drilldown executor
// ---------------------------------------------------------------------------

#[derive(Debug, FromQueryResult)]
struct DrilldownAggRow {
    group_key: String,
    label: String,
    total: f64,
}

/// Execute a drilldown for any registered schema.
///
/// Runs two GROUP BY queries (period 1 and period 2) and merges results.
pub async fn execute_drilldown(req: &DrilldownRequest) -> Result<DrilldownResponse> {
    let registry = get_registry();
    let table_name = registry
        .get_table_name(&req.schema_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown schema: {}", req.schema_id))?;

    let schema = registry
        .get_schema(&req.schema_id)
        .ok_or_else(|| anyhow::anyhow!("Schema not found: {}", req.schema_id))?;

    // Find the field definition for group_by
    let group_field = schema
        .fields
        .iter()
        .find(|f| f.id == req.group_by)
        .ok_or_else(|| anyhow::anyhow!("Unknown group_by field: {}", req.group_by))?
        .clone();

    let group_by_label = group_field.name.clone();

    // Find metric label
    let metric_field = schema
        .fields
        .iter()
        .find(|f| f.db_column == req.metric_column || f.id == req.metric_column);
    let metric_label = metric_field
        .map(|f| f.name.clone())
        .unwrap_or_else(|| req.metric_column.clone());

    // Period 2: explicit or auto-shift
    let (p2_from, p2_to) = match (&req.period2_from, &req.period2_to) {
        (Some(f), Some(t)) => (f.clone(), t.clone()),
        _ => (
            shift_date(&req.date_from, -1),
            shift_date(&req.date_to, -1),
        ),
    };

    let period1_label = period_label(&req.date_from, &req.date_to);
    let period2_label = period_label(&p2_from, &p2_to);

    // Build the SQL for one period
    let rows1 = fetch_drilldown_period(
        &table_name,
        &group_field.db_column,
        group_field.ref_table.as_deref(),
        group_field.ref_display_column.as_deref(),
        &req.metric_column,
        &req.date_from,
        &req.date_to,
        &req.connection_mp_refs,
    )
    .await?;

    let rows2 = fetch_drilldown_period(
        &table_name,
        &group_field.db_column,
        group_field.ref_table.as_deref(),
        group_field.ref_display_column.as_deref(),
        &req.metric_column,
        &p2_from,
        &p2_to,
        &req.connection_mp_refs,
    )
    .await?;

    // Merge by group_key
    let mut merged: std::collections::HashMap<String, DrilldownRow> =
        std::collections::HashMap::new();

    for r in rows1 {
        merged
            .entry(r.group_key.clone())
            .or_insert_with(|| DrilldownRow {
                group_key: r.group_key.clone(),
                label: r.label.clone(),
                value1: 0.0,
                value2: 0.0,
                delta_pct: None,
            })
            .value1 = r.total;
    }

    for r in rows2 {
        let entry = merged
            .entry(r.group_key.clone())
            .or_insert_with(|| DrilldownRow {
                group_key: r.group_key.clone(),
                label: r.label.clone(),
                value1: 0.0,
                value2: 0.0,
                delta_pct: None,
            });
        entry.value2 = r.total;
    }

    // Calculate deltas and sort by value1 DESC
    let mut rows: Vec<DrilldownRow> = merged
        .into_values()
        .map(|mut r| {
            r.delta_pct = pct_change(r.value1, r.value2);
            r
        })
        .collect();

    rows.sort_by(|a, b| b.value1.partial_cmp(&a.value1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(DrilldownResponse {
        rows,
        group_by_label,
        period1_label,
        period2_label,
        metric_label,
    })
}

#[allow(clippy::too_many_arguments)]
async fn fetch_drilldown_period(
    table_name: &str,
    group_col: &str,
    ref_table: Option<&str>,
    ref_display_col: Option<&str>,
    metric_col: &str,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<Vec<DrilldownAggRow>> {
    let db = get_connection();

    // Determine metric expression
    let metric_expr = if metric_col == "order_count" || metric_col == "registrator_ref" {
        "CAST(COUNT(DISTINCT t.registrator_ref) AS REAL)".to_string()
    } else {
        format!("CAST(COALESCE(SUM(t.{}), 0) AS REAL)", metric_col)
    };

    // Determine label expression (use JOIN for ref fields)
    let (select_label, join_clause) = if let (Some(rt), Some(rdc)) = (ref_table, ref_display_col) {
        let alias = "ref_t";
        (
            format!("COALESCE({alias}.{rdc}, t.{group_col})"),
            format!("LEFT JOIN {rt} {alias} ON t.{group_col} = {alias}.id"),
        )
    } else {
        (format!("t.{group_col}"), String::new())
    };

    let mut sql = format!(
        r#"
        SELECT
            COALESCE(t.{group_col}, '') AS group_key,
            COALESCE({select_label}, '') AS label,
            {metric_expr} AS total
        FROM {table_name} t
        {join_clause}
        WHERE t.date >= ? AND t.date <= ?
        "#,
    );

    let mut params: Vec<sea_orm::Value> =
        vec![date_from.to_string().into(), date_to.to_string().into()];

    if !connection_mp_refs.is_empty() {
        let placeholders: Vec<&str> = connection_mp_refs.iter().map(|_| "?").collect();
        sql.push_str(&format!(
            " AND t.connection_mp_ref IN ({})",
            placeholders.join(", ")
        ));
        for r in connection_mp_refs {
            params.push(r.clone().into());
        }
    }

    sql.push_str(&format!(" GROUP BY t.{group_col} ORDER BY total DESC"));

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    let rows = DrilldownAggRow::find_by_statement(stmt).all(db).await?;

    Ok(rows)
}
