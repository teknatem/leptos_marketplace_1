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

    // === COST (Себестоимость) ===
    let cost_data = repository::get_cost_by_marketplace_and_org(&date_from, &date_to).await?;
    let cost_rows = build_indicator_rows(
        &cost_data.iter().map(|r| (r.marketplace_code.clone(), r.organization_name.clone(), r.total_cost)).collect::<Vec<_>>(),
        "cost",
        "Себестоимость",
        &marketplaces,
        year,
        month,
    );
    rows.extend(cost_rows);

    // === RESULT (Результат) ===
    // Calculate: revenue + returns + cost for each marketplace/org
    let result_rows = build_result_rows(
        &revenue_data,
        &returns_data,
        &cost_data,
        &marketplaces,
        year,
        month,
    );
    rows.extend(result_rows);

    Ok(MonthlySummaryResponse {
        period,
        rows,
        marketplaces,
    })
}

/// Get available periods for the dashboard (YYYY-MM)
pub async fn get_available_periods() -> Result<Vec<String>> {
    repository::get_available_periods().await
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
    if data.is_empty() {
        return Vec::new();
    }

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

/// Build result indicator rows by combining revenue, returns, and cost
fn build_result_rows(
    revenue_data: &[repository::RevenueAggregation],
    returns_data: &[repository::ReturnsAggregation],
    cost_data: &[repository::CostAggregation],
    marketplaces: &[String],
    year: i32,
    month: u32,
) -> Vec<IndicatorRow> {
    let mut rows = Vec::new();

    // Build maps for easy lookup
    let mut revenue_map: HashMap<(String, String), f64> = HashMap::new();
    let mut returns_map: HashMap<(String, String), f64> = HashMap::new();
    let mut cost_map: HashMap<(String, String), f64> = HashMap::new();
    let mut all_orgs: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Populate revenue map
    for r in revenue_data {
        let mp = r.marketplace_code.clone().unwrap_or_else(|| "Другое".to_string());
        if let Some(org) = &r.organization_name {
            if !org.is_empty() {
                all_orgs.insert(org.clone());
                revenue_map.insert((org.clone(), mp), r.total_revenue);
            }
        }
    }

    // Populate returns map
    for r in returns_data {
        let mp = r.marketplace_code.clone().unwrap_or_else(|| "Другое".to_string());
        if let Some(org) = &r.organization_name {
            if !org.is_empty() {
                all_orgs.insert(org.clone());
                returns_map.insert((org.clone(), mp), r.total_returns);
            }
        }
    }

    // Populate cost map
    for r in cost_data {
        let mp = r.marketplace_code.clone().unwrap_or_else(|| "Другое".to_string());
        if let Some(org) = &r.organization_name {
            if !org.is_empty() {
                all_orgs.insert(org.clone());
                cost_map.insert((org.clone(), mp), r.total_cost);
            }
        }
    }

    // Calculate totals by marketplace
    let mut mp_totals: HashMap<String, f64> = HashMap::new();
    let mut grand_total = 0.0;

    for mp in marketplaces {
        let mut mp_total = 0.0;
        
        // Sum revenue for this marketplace
        for r in revenue_data {
            if r.marketplace_code.as_ref() == Some(mp) {
                mp_total += r.total_revenue;
            }
        }
        
        // Add returns for this marketplace
        for r in returns_data {
            if r.marketplace_code.as_ref() == Some(mp) {
                mp_total += r.total_returns;
            }
        }
        
        // Add cost for this marketplace
        for c in cost_data {
            if c.marketplace_code.as_ref() == Some(mp) {
                mp_total += c.total_cost;
            }
        }
        
        mp_totals.insert(mp.clone(), mp_total);
        grand_total += mp_total;
    }

    // Build total row (level 0)
    let mut total_values: HashMap<String, f64> = HashMap::new();
    for mp in marketplaces {
        total_values.insert(mp.clone(), *mp_totals.get(mp).unwrap_or(&0.0));
    }
    total_values.insert("total".to_string(), grand_total);

    let total_row = IndicatorRow {
        indicator_id: "result".to_string(),
        indicator_name: "Результат".to_string(),
        group_name: None,
        level: 0,
        values: total_values,
        drilldown_filter: DrilldownFilter::for_month(year, month),
    };
    rows.push(total_row);

    // Build organization breakdown rows
    let mut org_names: Vec<String> = all_orgs.into_iter().collect();
    org_names.sort();

    for org_name in org_names {
        let mut org_values: HashMap<String, f64> = HashMap::new();
        let mut org_total = 0.0;

        for mp in marketplaces {
            let key = (org_name.clone(), mp.clone());
            let revenue = *revenue_map.get(&key).unwrap_or(&0.0);
            let returns = *returns_map.get(&key).unwrap_or(&0.0);
            let cost = *cost_map.get(&key).unwrap_or(&0.0);
            let result = revenue + returns + cost;
            
            org_values.insert(mp.clone(), result);
            org_total += result;
        }
        org_values.insert("total".to_string(), org_total);

        let org_row = IndicatorRow {
            indicator_id: "result".to_string(),
            indicator_name: "Результат".to_string(),
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

