use anyhow::Result;
use contracts::shared::drilldown::{DrilldownRequest, DrilldownResponse, DrilldownRow};
use sea_orm::{FromQueryResult, Statement};

use crate::shared::data::db::get_connection;
use crate::shared::universal_dashboard::get_registry;

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

fn period_label(date_from: &str, date_to: &str) -> String {
    let months = [
        "Янв", "Фев", "Мар", "Апр", "Май", "Июн", "Июл", "Авг", "Сен", "Окт", "Ноя", "Дек",
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

#[derive(Debug, FromQueryResult)]
struct DrilldownAggRow {
    group_key: String,
    label: String,
    total: f64,
}

pub async fn execute_schema_drilldown(req: &DrilldownRequest) -> Result<DrilldownResponse> {
    let registry = get_registry();
    let table_name = registry
        .get_table_name(&req.schema_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown schema: {}", req.schema_id))?;

    let schema = registry
        .get_schema(&req.schema_id)
        .ok_or_else(|| anyhow::anyhow!("Schema not found: {}", req.schema_id))?;

    let group_field = schema
        .fields
        .iter()
        .find(|f| f.id == req.group_by)
        .ok_or_else(|| anyhow::anyhow!("Unknown group_by field: {}", req.group_by))?
        .clone();

    let group_by_label = group_field.name.clone();

    let metric_field = schema
        .fields
        .iter()
        .find(|f| f.db_column == req.metric_column || f.id == req.metric_column);
    let metric_label = metric_field
        .map(|f| f.name.clone())
        .unwrap_or_else(|| req.metric_column.clone());

    let (p2_from, p2_to) = match (&req.period2_from, &req.period2_to) {
        (Some(f), Some(t)) => (f.clone(), t.clone()),
        _ => (shift_date(&req.date_from, -1), shift_date(&req.date_to, -1)),
    };

    let period1_label = period_label(&req.date_from, &req.date_to);
    let period2_label = period_label(&p2_from, &p2_to);

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
                metric_values: std::collections::HashMap::new(),
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
                metric_values: std::collections::HashMap::new(),
            });
        entry.value2 = r.total;
    }

    let mut rows: Vec<DrilldownRow> = merged
        .into_values()
        .map(|mut r| {
            r.delta_pct = pct_change(r.value1, r.value2);
            r
        })
        .collect();

    rows.sort_by(|a, b| {
        b.value1
            .partial_cmp(&a.value1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(DrilldownResponse {
        rows,
        group_by_label,
        period1_label,
        period2_label,
        metric_label,
        metric_columns: vec![],
        selected_dimension: None,
        coverage: None,
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

    let metric_expr = if metric_col == "order_count" || metric_col == "registrator_ref" {
        "CAST(COUNT(DISTINCT t.registrator_ref) AS REAL)".to_string()
    } else {
        format!("CAST(COALESCE(SUM(t.{}), 0) AS REAL)", metric_col)
    };

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
        WHERE substr(t.date, 1, 10) >= ? AND substr(t.date, 1, 10) <= ?
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
