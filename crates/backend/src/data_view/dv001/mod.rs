//! dv001 — DataView: Продажи (2 периода)
//!
//! Источник данных: p904_sales_data
//! Поддерживаемые метрики (params["metric"]):
//!   revenue    = customer_in + customer_out   (по умолчанию)
//!   cost       = cost
//!   commission = commission_out
//!   expenses   = acquiring_out + penalty_out + logistics_out
//!   profit     = seller_out

use anyhow::Result;
use contracts::shared::data_view::ViewContext;
use contracts::shared::drilldown::{DrilldownResponse, DrilldownRow};
use contracts::shared::indicators::{IndicatorId, IndicatorStatus, IndicatorValue};
use sea_orm::{FromQueryResult, Statement};

use crate::shared::data::db::get_connection;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extracts day-of-month as zero-padded string from "YYYY-MM-DD" ("01".."31").
fn fmt_day_label(iso: &str) -> String {
    let date_part = iso.split('T').next().unwrap_or(iso);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() >= 3 {
        parts[2]
            .parse::<u32>()
            .map(|d| format!("{:02}", d))
            .unwrap_or_else(|_| iso.to_string())
    } else {
        iso.to_string()
    }
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
        _ => (
            shift_date(&ctx.date_from, -1),
            shift_date(&ctx.date_to, -1),
        ),
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
        if to_parts.get(1) == Some(&parts[1]) && to_parts.get(0) == Some(&parts[0]) {
            format!("{} {}", month_name, y)
        } else {
            format!("{} – {}", date_from, date_to)
        }
    } else {
        format!("{} – {}", date_from, date_to)
    }
}

fn pct_change(cur: f64, prev: f64) -> Option<f64> {
    if prev.abs() < 0.01 {
        None
    } else {
        Some(((cur - prev) / prev.abs()) * 100.0)
    }
}

fn status_by_change(change: Option<f64>) -> IndicatorStatus {
    match change {
        Some(c) if c > 5.0 => IndicatorStatus::Good,
        Some(c) if c < -5.0 => IndicatorStatus::Bad,
        _ => IndicatorStatus::Neutral,
    }
}

// ---------------------------------------------------------------------------
// Metric resolution
// ---------------------------------------------------------------------------

/// Returns (sql_expression, display_label) for the requested metric.
fn resolve_metric(ctx: &ViewContext) -> (&'static str, &'static str) {
    match ctx
        .params
        .get("metric")
        .map(|s| s.as_str())
        .unwrap_or("revenue")
    {
        "cost" => ("COALESCE(cost, 0)", "Себестоимость"),
        "commission" => ("COALESCE(commission_out, 0)", "Комиссия"),
        "expenses" => (
            "COALESCE(acquiring_out, 0) + COALESCE(penalty_out, 0) + COALESCE(logistics_out, 0)",
            "Расходы",
        ),
        "profit" => ("COALESCE(seller_out, 0)", "Прибыль"),
        _ => (
            "COALESCE(customer_in, 0) + COALESCE(customer_out, 0)",
            "Выручка",
        ),
    }
}

// ---------------------------------------------------------------------------
// Internal DB row types
// ---------------------------------------------------------------------------

#[derive(Debug, FromQueryResult)]
struct AggRow {
    total: f64,
}

#[derive(Debug, FromQueryResult)]
struct DailyRow {
    #[allow(dead_code)]
    day_offset: String,
    #[allow(dead_code)]
    day_label: String,
    total: f64,
}

#[derive(Debug, FromQueryResult)]
struct DrilldownAggRow {
    group_key: String,
    label: String,
    total: f64,
}

// ---------------------------------------------------------------------------
// Internal fetch functions
// ---------------------------------------------------------------------------

/// Fetch aggregate total for a single period (used for P2 previous_value).
async fn fetch_aggregate(
    metric_expr: &str,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<f64> {
    let db = get_connection();

    let mut sql = format!(
        r#"
        SELECT CAST(COALESCE(SUM({metric_expr}), 0) AS REAL) AS total
        FROM p904_sales_data p
        WHERE p.date >= ? AND p.date <= ?
        "#
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
    let row = AggRow::find_by_statement(stmt)
        .one(db)
        .await?
        .unwrap_or(AggRow { total: 0.0 });

    Ok(row.total)
}

/// Fetch daily rows for a period, sorted by date ascending.
///
/// Returns one row per day that has data. Used for both scalar total (via sum)
/// and sparkline points.
async fn fetch_daily_rows(
    metric_expr: &str,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<Vec<DailyRow>> {
    let db = get_connection();

    let mut sql = format!(
        r#"
        SELECT
            printf('%06d', CAST(julianday(DATE(t.date)) - julianday(?) AS INTEGER)) AS day_offset,
            DATE(t.date) AS day_label,
            CAST(COALESCE(SUM({metric_expr}), 0) AS REAL) AS total
        FROM p904_sales_data t
        WHERE t.date >= ? AND t.date <= ?
        "#
    );

    let mut params: Vec<sea_orm::Value> = vec![
        date_from.to_string().into(), // offset base
        date_from.to_string().into(),
        date_to.to_string().into(),
    ];

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

    sql.push_str(" GROUP BY DATE(t.date) ORDER BY day_offset ASC");

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    Ok(DailyRow::find_by_statement(stmt).all(db).await?)
}

/// Fetch drilldown aggregation for a single period.
///
/// For `dim1`..`dim6` dimensions (stored in `a004_nomenclature`), performs
/// a LEFT JOIN on `nomenclature_ref` and groups by the dimension column.
async fn fetch_drilldown_period(
    group_col: &str,
    metric_expr: &str,
    ref_table: Option<&str>,
    ref_display_col: Option<&str>,
    source_table: Option<&str>,
    join_on_col: Option<&str>,
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<Vec<DrilldownAggRow>> {
    let db = get_connection();

    // Date grouping: use day-offset as group_key so P1 and P2 rows align correctly.
    if group_col == "date" {
        let mut sql = format!(
            r#"
            SELECT
                printf('%06d', CAST(julianday(DATE(t.date)) - julianday(?) AS INTEGER)) AS group_key,
                DATE(t.date) AS label,
                CAST(COALESCE(SUM({metric_expr}), 0) AS REAL) AS total
            FROM p904_sales_data t
            WHERE t.date >= ? AND t.date <= ?
            "#
        );

        let mut params: Vec<sea_orm::Value> = vec![
            date_from.to_string().into(),
            date_from.to_string().into(),
            date_to.to_string().into(),
        ];

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

        sql.push_str(" GROUP BY DATE(t.date) ORDER BY group_key ASC");

        let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
        return Ok(DrilldownAggRow::find_by_statement(stmt).all(db).await?);
    }

    // Dimension grouping from a joined table (e.g. dim1..dim6 from a004_nomenclature).
    if let (Some(src_tbl), Some(join_col)) = (source_table, join_on_col) {
        let alias = "dim_t";
        let mut sql = format!(
            r#"
            SELECT
                COALESCE({alias}.{group_col}, '') AS group_key,
                COALESCE({alias}.{group_col}, '') AS label,
                CAST(COALESCE(SUM({metric_expr}), 0) AS REAL) AS total
            FROM p904_sales_data t
            LEFT JOIN {src_tbl} {alias} ON t.{join_col} = {alias}.id
            WHERE t.date >= ? AND t.date <= ?
            "#
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

        sql.push_str(&format!(
            " GROUP BY {alias}.{group_col} ORDER BY total DESC"
        ));

        let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
        return Ok(DrilldownAggRow::find_by_statement(stmt).all(db).await?);
    }

    // Standard dimension grouping (field is directly on p904_sales_data).
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
            CAST(COALESCE(SUM({metric_expr}), 0) AS REAL) AS total
        FROM p904_sales_data t
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
    Ok(DrilldownAggRow::find_by_statement(stmt).all(db).await?)
}

// ---------------------------------------------------------------------------
// Public: compute_scalar
// ---------------------------------------------------------------------------

/// Вычислить скалярное значение метрики за 2 периода с sparkline данными.
///
/// P1 данные всегда запрашиваются по дням (для sparkline + сумма = scalar).
/// P2 — агрегат одним запросом.
/// Оба запроса выполняются параллельно через `tokio::join!`.
pub async fn compute_scalar(ctx: &ViewContext) -> Result<IndicatorValue> {
    let (metric_expr, _metric_label) = resolve_metric(ctx);
    let (p2_from, p2_to) = resolve_period2(ctx);

    let (daily_result, prev_result) = tokio::join!(
        fetch_daily_rows(metric_expr, &ctx.date_from, &ctx.date_to, &ctx.connection_mp_refs),
        fetch_aggregate(metric_expr, &p2_from, &p2_to, &ctx.connection_mp_refs),
    );

    let daily = daily_result?;
    let prev = prev_result?;

    let spark_points: Vec<f64> = daily.iter().map(|r| r.total).collect();
    let cur: f64 = spark_points.iter().sum();
    let change = pct_change(cur, prev);

    Ok(IndicatorValue {
        id: IndicatorId::new("dv001_revenue"),
        value: Some(cur),
        previous_value: Some(prev),
        change_percent: change,
        status: status_by_change(change),
        subtitle: None,
        spark_points,
    })
}

// ---------------------------------------------------------------------------
// Public: compute_drilldown
// ---------------------------------------------------------------------------

/// Вычислить детализацию по измерению за 2 периода.
///
/// `group_by` — field_id из схемы ds03_p904_sales.
/// Данные для обоих периодов запрашиваются всегда по дням при group_by == "date",
/// иначе — агрегаты по выбранному измерению.
pub async fn compute_drilldown(ctx: &ViewContext, group_by: &str) -> Result<DrilldownResponse> {
    use crate::shared::universal_dashboard::get_registry;

    let registry = get_registry();
    let schema = registry
        .get_schema("ds03_p904_sales")
        .ok_or_else(|| anyhow::anyhow!("Schema ds03_p904_sales not found"))?;

    let group_field = schema
        .fields
        .iter()
        .find(|f| f.id == group_by)
        .ok_or_else(|| anyhow::anyhow!("Unknown group_by field: {}", group_by))?
        .clone();

    let (metric_expr, metric_label) = resolve_metric(ctx);
    let group_by_label = group_field.name.clone();

    let (p2_from, p2_to) = resolve_period2(ctx);
    let period1_label = period_label(&ctx.date_from, &ctx.date_to);
    let period2_label = period_label(&p2_from, &p2_to);

    tracing::info!(
        "drilldown: group_by={} metric={} | P1 {} .. {} | P2 {} .. {}",
        group_by,
        metric_expr,
        ctx.date_from,
        ctx.date_to,
        p2_from,
        p2_to
    );

    let (rows1_result, rows2_result) = tokio::join!(
        fetch_drilldown_period(
            &group_field.db_column,
            metric_expr,
            group_field.ref_table.as_deref(),
            group_field.ref_display_column.as_deref(),
            group_field.source_table.as_deref(),
            group_field.join_on_column.as_deref(),
            &ctx.date_from,
            &ctx.date_to,
            &ctx.connection_mp_refs,
        ),
        fetch_drilldown_period(
            &group_field.db_column,
            metric_expr,
            group_field.ref_table.as_deref(),
            group_field.ref_display_column.as_deref(),
            group_field.source_table.as_deref(),
            group_field.join_on_column.as_deref(),
            &p2_from,
            &p2_to,
            &ctx.connection_mp_refs,
        ),
    );

    let rows1 = rows1_result?;
    let rows2 = rows2_result?;

    tracing::info!(
        "drilldown: rows1={} rows2={} | sample_keys1={:?} sample_keys2={:?}",
        rows1.len(),
        rows2.len(),
        rows1.iter().take(3).map(|r| r.group_key.as_str()).collect::<Vec<_>>(),
        rows2.iter().take(3).map(|r| r.group_key.as_str()).collect::<Vec<_>>(),
    );

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

    let is_date_group = group_by == "date";

    let mut rows: Vec<DrilldownRow> = merged
        .into_values()
        .map(|mut r| {
            r.delta_pct = pct_change(r.value1, r.value2);
            if is_date_group {
                r.label = fmt_day_label(&r.label);
            }
            r
        })
        .collect();

    if is_date_group {
        rows.sort_by(|a, b| a.group_key.cmp(&b.group_key));
    } else {
        rows.sort_by(|a, b| b.value1.partial_cmp(&a.value1).unwrap_or(std::cmp::Ordering::Equal));
    }

    Ok(DrilldownResponse {
        rows,
        group_by_label,
        period1_label,
        period2_label,
        metric_label: metric_label.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Статические метаданные этого DataView (из embedded JSON).
pub fn meta() -> contracts::shared::data_view::DataViewMeta {
    const JSON: &str = include_str!("metadata.json");
    serde_json::from_str(JSON).expect("dv001/metadata.json parse error")
}
