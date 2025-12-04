use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request for monthly summary dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySummaryRequest {
    pub year: i32,
    pub month: u32,
}

/// Response for monthly summary dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlySummaryResponse {
    /// Period in format "YYYY-MM"
    pub period: String,
    /// List of indicator rows
    pub rows: Vec<IndicatorRow>,
    /// List of marketplace codes (e.g., ["WB", "OZON", "YM"])
    pub marketplaces: Vec<String>,
}

/// Single indicator row in the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorRow {
    /// Indicator identifier (e.g., "revenue")
    pub indicator_id: String,
    /// Display name (e.g., "Выручка")
    pub indicator_name: String,
    /// Group/organization name, None for total row
    pub group_name: Option<String>,
    /// Hierarchy level (0 = total, 1 = organization detail)
    pub level: u32,
    /// Values by marketplace code + "total" key
    /// e.g., {"WB": 1000.0, "OZON": 2000.0, "YM": 500.0, "total": 3500.0}
    pub values: HashMap<String, f64>,
    /// Filter for drill-down navigation
    pub drilldown_filter: DrilldownFilter,
}

/// Filter parameters for drill-down navigation to p904_sales_data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrilldownFilter {
    /// Start date in format "YYYY-MM-DD"
    pub date_from: String,
    /// End date in format "YYYY-MM-DD"
    pub date_to: String,
    /// Marketplace type filter (optional)
    pub marketplace_type: Option<String>,
    /// Organization reference filter (optional)
    pub organization_ref: Option<String>,
}

impl DrilldownFilter {
    /// Create a new drilldown filter for a month
    pub fn for_month(year: i32, month: u32) -> Self {
        let date_from = format!("{:04}-{:02}-01", year, month);
        let date_to = Self::last_day_of_month(year, month);
        Self {
            date_from,
            date_to,
            marketplace_type: None,
            organization_ref: None,
        }
    }

    /// Create a drilldown filter for a specific marketplace
    pub fn for_marketplace(year: i32, month: u32, marketplace_type: &str) -> Self {
        let mut filter = Self::for_month(year, month);
        filter.marketplace_type = Some(marketplace_type.to_string());
        filter
    }

    /// Create a drilldown filter for a specific organization
    pub fn for_organization(year: i32, month: u32, organization_ref: &str) -> Self {
        let mut filter = Self::for_month(year, month);
        filter.organization_ref = Some(organization_ref.to_string());
        filter
    }

    /// Create a drilldown filter for a specific marketplace and organization
    pub fn for_marketplace_and_organization(
        year: i32,
        month: u32,
        marketplace_type: &str,
        organization_ref: &str,
    ) -> Self {
        let mut filter = Self::for_month(year, month);
        filter.marketplace_type = Some(marketplace_type.to_string());
        filter.organization_ref = Some(organization_ref.to_string());
        filter
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
}

