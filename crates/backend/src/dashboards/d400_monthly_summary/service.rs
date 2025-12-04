use anyhow::Result;
use contracts::dashboards::d400_monthly_summary::{
    DrilldownFilter, IndicatorRow, MonthlySummaryRequest, MonthlySummaryResponse,
};
use std::collections::HashMap;

use super::repository;

/// Marketplace type to display code mapping
fn marketplace_display_code(marketplace_type: Option<&str>) -> String {
    match marketplace_type {
        Some("Wildberries") => "WB".to_string(),
        Some("Озон") => "OZON".to_string(),
        Some("Яндекс.Маркет") => "YM".to_string(),
        Some(other) => other.to_string(),
        None => "Другое".to_string(),
    }
}

/// Get monthly summary data
pub async fn get_monthly_summary(request: MonthlySummaryRequest) -> Result<MonthlySummaryResponse> {
    let year = request.year;
    let month = request.month;

    // Calculate date range for the month
    let date_from = format!("{:04}-{:02}-01", year, month);
    let date_to = last_day_of_month(year, month);
    let period = format!("{:04}-{:02}", year, month);

    // Get aggregated revenue data
    let revenue_data = repository::get_revenue_by_marketplace_and_org(&date_from, &date_to).await?;

    // Get active marketplaces (hardcoded order: WB, OZON, YM)
    let marketplaces = vec!["WB".to_string(), "OZON".to_string(), "YM".to_string()];

    // Build indicator rows
    let mut rows = Vec::new();

    // Calculate totals by marketplace and organization
    let mut mp_totals: HashMap<String, f64> = HashMap::new();
    let mut org_mp_totals: HashMap<(String, String, String), f64> = HashMap::new(); // (org_ref, org_name, mp_code) -> revenue
    let mut org_names: HashMap<String, String> = HashMap::new();

    for item in &revenue_data {
        let mp_code = marketplace_display_code(item.marketplace_type.as_deref());
        let revenue = item.total_revenue;

        // Add to marketplace total
        *mp_totals.entry(mp_code.clone()).or_insert(0.0) += revenue;

        // Add to organization-marketplace breakdown
        if let Some(org_ref) = &item.organization_ref {
            if !org_ref.is_empty() {
                let org_name = item
                    .organization_name
                    .clone()
                    .unwrap_or_else(|| "Неизвестная организация".to_string());
                org_names.insert(org_ref.clone(), org_name.clone());
                *org_mp_totals
                    .entry((org_ref.clone(), org_name, mp_code))
                    .or_insert(0.0) += revenue;
            }
        }
    }

    // Calculate grand total
    let grand_total: f64 = mp_totals.values().sum();

    // Build total row for revenue
    let mut total_values: HashMap<String, f64> = HashMap::new();
    for mp in &marketplaces {
        total_values.insert(mp.clone(), *mp_totals.get(mp).unwrap_or(&0.0));
    }
    total_values.insert("total".to_string(), grand_total);

    let total_row = IndicatorRow {
        indicator_id: "revenue".to_string(),
        indicator_name: "Выручка".to_string(),
        group_name: None,
        level: 0,
        values: total_values,
        drilldown_filter: DrilldownFilter::for_month(year, month as u32),
    };
    rows.push(total_row);

    // Build organization breakdown rows
    let mut org_refs: Vec<String> = org_names.keys().cloned().collect();
    org_refs.sort();

    for org_ref in org_refs {
        let org_name = org_names.get(&org_ref).cloned().unwrap_or_default();
        let mut org_values: HashMap<String, f64> = HashMap::new();
        let mut org_total = 0.0;

        for mp in &marketplaces {
            let key = (org_ref.clone(), org_name.clone(), mp.clone());
            let value = *org_mp_totals.get(&key).unwrap_or(&0.0);
            org_values.insert(mp.clone(), value);
            org_total += value;
        }
        org_values.insert("total".to_string(), org_total);

        let org_row = IndicatorRow {
            indicator_id: "revenue".to_string(),
            indicator_name: "Выручка".to_string(),
            group_name: Some(org_name.clone()),
            level: 1,
            values: org_values,
            drilldown_filter: DrilldownFilter::for_organization(year, month as u32, &org_ref),
        };
        rows.push(org_row);
    }

    Ok(MonthlySummaryResponse {
        period,
        rows,
        marketplaces,
    })
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

