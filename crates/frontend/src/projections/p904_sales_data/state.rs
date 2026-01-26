use chrono::{Datelike, Utc};
use contracts::projections::p904_sales_data::dto::SalesDataDto;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct SalesDataState {
    pub sales: Vec<SalesDataDto>,
    pub date_from: String,
    pub date_to: String,
    pub cabinet_filter: String,
    pub limit: String,
    pub sort_column: Option<String>, // Using String representation of enum for simplicity in state
    pub sort_ascending: bool,
    pub is_loaded: bool,
}

impl Default for SalesDataState {
    fn default() -> Self {
        // Default period: current month
        let now = Utc::now().date_naive();
        let year = now.year();
        let month = now.month();
        let month_start =
            chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid month start date");
        let month_end = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("Invalid month end date")
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("Invalid month end date")
        };

        Self {
            sales: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            cabinet_filter: "".to_string(),
            limit: "1000".to_string(),
            sort_column: None,
            sort_ascending: true,
            is_loaded: false,
        }
    }
}

// Create state within component scope instead of thread-local
// This ensures state is properly disposed when component unmounts
pub fn create_state() -> RwSignal<SalesDataState> {
    RwSignal::new(SalesDataState::default())
}
