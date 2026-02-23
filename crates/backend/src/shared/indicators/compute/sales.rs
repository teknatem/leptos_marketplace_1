use anyhow::Result;
use contracts::shared::indicators::*;
use sea_orm::{FromQueryResult, Statement};

use crate::shared::data::db::get_connection;

// ---------------------------------------------------------------------------
// Internal aggregation row
// ---------------------------------------------------------------------------

#[derive(Debug, FromQueryResult)]
struct SalesAgg {
    total_revenue: f64,
    total_returns: f64,
    order_count: i32,
}

async fn fetch_agg(ctx: &IndicatorContext) -> Result<SalesAgg> {
    let db = get_connection();

    let mut sql = String::from(
        r#"
        SELECT
            COALESCE(SUM(p.customer_in), 0)                            AS total_revenue,
            COALESCE(SUM(p.customer_out), 0)                           AS total_returns,
            CAST(COUNT(DISTINCT p.registrator_ref) AS INTEGER)         AS order_count
        FROM p904_sales_data p
        LEFT JOIN a006_connection_mp conn ON p.connection_mp_ref = conn.id
        LEFT JOIN a005_marketplace mp ON conn.marketplace = mp.id
        WHERE p.date >= ? AND p.date <= ?
    "#,
    );

    let mut params: Vec<sea_orm::Value> = vec![ctx.date_from.clone().into(), ctx.date_to.clone().into()];

    if let Some(ref org) = ctx.organization_ref {
        sql.push_str(" AND conn.organization_ref = ?");
        params.push(org.clone().into());
    }
    if let Some(ref mp) = ctx.marketplace {
        sql.push_str(
            " AND CASE mp.marketplace_type \
             WHEN 'mp-wb' THEN 'WB' \
             WHEN 'mp-ozon' THEN 'OZON' \
             WHEN 'mp-ym' THEN 'YM' \
             ELSE mp.marketplace_type END = ?",
        );
        params.push(mp.clone().into());
    }
    if !ctx.connection_mp_refs.is_empty() {
        let placeholders: Vec<&str> = ctx.connection_mp_refs.iter().map(|_| "?").collect();
        sql.push_str(&format!(
            " AND p.connection_mp_ref IN ({})",
            placeholders.join(", ")
        ));
        for r in &ctx.connection_mp_refs {
            params.push(r.clone().into());
        }
    }

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);
    let row = SalesAgg::find_by_statement(stmt)
        .one(db)
        .await?
        .unwrap_or(SalesAgg {
            total_revenue: 0.0,
            total_returns: 0.0,
            order_count: 0,
        });

    Ok(row)
}

/// Shift `IndicatorContext` back by the same period length for comparison.
fn previous_period(ctx: &IndicatorContext) -> IndicatorContext {
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

    IndicatorContext {
        date_from: shift_date(&ctx.date_from, -1),
        date_to: shift_date(&ctx.date_to, -1),
        organization_ref: ctx.organization_ref.clone(),
        marketplace: ctx.marketplace.clone(),
        connection_mp_refs: ctx.connection_mp_refs.clone(),
        extra: ctx.extra.clone(),
    }
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

// ---------------------------------------------------------------------------
// Public compute functions
// ---------------------------------------------------------------------------

pub async fn compute_sales_revenue(ctx: &IndicatorContext) -> Result<IndicatorValue> {
    let cur = fetch_agg(ctx).await?;
    let prev = fetch_agg(&previous_period(ctx)).await?;
    let change = pct_change(cur.total_revenue, prev.total_revenue);
    Ok(IndicatorValue {
        id: crate::shared::indicators::metadata::ids::sales_revenue(),
        value: Some(cur.total_revenue),
        previous_value: Some(prev.total_revenue),
        change_percent: change,
        status: status_by_change(change, true),
        subtitle: None,
    })
}

pub async fn compute_sales_order_count(ctx: &IndicatorContext) -> Result<IndicatorValue> {
    let cur = fetch_agg(ctx).await?;
    let prev = fetch_agg(&previous_period(ctx)).await?;
    let change = pct_change(cur.order_count as f64, prev.order_count as f64);
    Ok(IndicatorValue {
        id: crate::shared::indicators::metadata::ids::sales_order_count(),
        value: Some(cur.order_count as f64),
        previous_value: Some(prev.order_count as f64),
        change_percent: change,
        status: status_by_change(change, true),
        subtitle: None,
    })
}

pub async fn compute_sales_avg_check(ctx: &IndicatorContext) -> Result<IndicatorValue> {
    let cur = fetch_agg(ctx).await?;
    let prev = fetch_agg(&previous_period(ctx)).await?;

    let cur_avg = if cur.order_count > 0 {
        cur.total_revenue / cur.order_count as f64
    } else {
        0.0
    };
    let prev_avg = if prev.order_count > 0 {
        prev.total_revenue / prev.order_count as f64
    } else {
        0.0
    };
    let change = pct_change(cur_avg, prev_avg);

    Ok(IndicatorValue {
        id: crate::shared::indicators::metadata::ids::sales_avg_check(),
        value: Some(cur_avg),
        previous_value: Some(prev_avg),
        change_percent: change,
        status: status_by_change(change, true),
        subtitle: None,
    })
}

pub async fn compute_sales_returns_sum(ctx: &IndicatorContext) -> Result<IndicatorValue> {
    let cur = fetch_agg(ctx).await?;
    let prev = fetch_agg(&previous_period(ctx)).await?;
    let change = pct_change(cur.total_returns.abs(), prev.total_returns.abs());
    Ok(IndicatorValue {
        id: crate::shared::indicators::metadata::ids::sales_returns_sum(),
        value: Some(cur.total_returns),
        previous_value: Some(prev.total_returns),
        change_percent: change,
        status: status_by_change(change, false),
        subtitle: None,
    })
}
