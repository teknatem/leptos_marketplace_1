use anyhow::Result;
use contracts::dashboards::d400_monthly_summary::{
    DrilldownFilter, IndicatorRow, MonthlySummaryRequest, MonthlySummaryResponse,
};
use std::collections::HashMap;

use super::repository;

/// Get monthly summary data
pub async fn get_monthly_summary(request: MonthlySummaryRequest) -> Result<MonthlySummaryResponse> {
    let year = request.year;
    let month = request.month;

    // Calculate date range for the month
    let date_from = format!("{:04}-{:02}-01", year, month);
    let date_to = last_day_of_month(year, month);
    let period = format!("{:04}-{:02}", year, month);

    // Fixed marketplaces order: WB, OZON, YM
    let marketplaces = vec!["WB".to_string(), "OZON".to_string(), "YM".to_string()];

    // Build indicator rows
    let mut rows = Vec::new();

    // === REVENUE (Выручка) ===
    let revenue_data = repository::get_revenue_by_marketplace_and_org(&date_from, &date_to).await?;
    let revenue_rows = build_indicator_rows(
        &revenue_data.iter().map(|r| (r.marketplace_code.clone(), r.organization_name.clone(), r.total_revenue)).collect::<Vec<_>>(),
        "revenue",
        "Выручка",
        &marketplaces,
        year,
        month,
    );
    rows.extend(revenue_rows);

    // === RETURNS (Возвраты) ===
    let returns_data = repository::get_returns_by_marketplace_and_org(&date_from, &date_to).await?;
    let returns_rows = build_indicator_rows(
        &returns_data.iter().map(|r| (r.marketplace_code.clone(), r.organization_name.clone(), r.total_returns)).collect::<Vec<_>>(),
        "returns",
        "Возвраты",
        &marketplaces,
        year,
        month,
    );
    rows.extend(returns_rows);

    Ok(MonthlySummaryResponse {
        period,
        rows,
        marketplaces,
    })
}

/// Build indicator rows from aggregated data
fn build_indicator_rows(
    data: &[(Option<String>, Option<String>, f64)], // (marketplace_code, org_name, value)
    indicator_id: &str,
    indicator_name: &str,
    marketplaces: &[String],
    year: i32,
    month: u32,
) -> Vec<IndicatorRow> {
    let mut rows = Vec::new();

    // Calculate totals by marketplace and organization
    let mut mp_totals: HashMap<String, f64> = HashMap::new();
    let mut org_mp_totals: HashMap<(String, String), f64> = HashMap::new();
    let mut org_names: Vec<String> = Vec::new();

    for (mp_code_opt, org_name_opt, value) in data {
        let mp_code = mp_code_opt.clone().unwrap_or_else(|| "Другое".to_string());

        // Add to marketplace total
        *mp_totals.entry(mp_code.clone()).or_insert(0.0) += value;

        // Add to organization-marketplace breakdown
        if let Some(org_name) = org_name_opt {
            if !org_name.is_empty() {
                if !org_names.contains(org_name) {
                    org_names.push(org_name.clone());
                }
                *org_mp_totals
                    .entry((org_name.clone(), mp_code))
                    .or_insert(0.0) += value;
            }
        }
    }

    // Calculate grand total
    let grand_total: f64 = mp_totals.values().sum();

    // Build total row
    let mut total_values: HashMap<String, f64> = HashMap::new();
    for mp in marketplaces {
        total_values.insert(mp.clone(), *mp_totals.get(mp).unwrap_or(&0.0));
    }
    total_values.insert("total".to_string(), grand_total);

    let total_row = IndicatorRow {
        indicator_id: indicator_id.to_string(),
        indicator_name: indicator_name.to_string(),
        group_name: None,
        level: 0,
        values: total_values,
        drilldown_filter: DrilldownFilter::for_month(year, month),
    };
    rows.push(total_row);

    // Build organization breakdown rows (sorted alphabetically)
    org_names.sort();

    for org_name in org_names {
        let mut org_values: HashMap<String, f64> = HashMap::new();
        let mut org_total = 0.0;

        for mp in marketplaces {
            let key = (org_name.clone(), mp.clone());
            let value = *org_mp_totals.get(&key).unwrap_or(&0.0);
            org_values.insert(mp.clone(), value);
            org_total += value;
        }
        org_values.insert("total".to_string(), org_total);

        let org_row = IndicatorRow {
            indicator_id: indicator_id.to_string(),
            indicator_name: indicator_name.to_string(),
            group_name: Some(org_name.clone()),
            level: 1,
            values: org_values,
            drilldown_filter: DrilldownFilter::for_organization(year, month, &org_name),
        };
        rows.push(org_row);
    }

    rows
}

/// Calculate the last day of a month
fn last_day_of_month(year: i32, month: u32) -> String {
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    };
    format!("{:04}-{:02}-{:02}", year, month, days)
}

