use super::SalesRegisterDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct SalesRegisterState {
    pub sales: Vec<SalesRegisterDto>,
    pub date_from: String,
    pub date_to: String,
    pub marketplace: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub is_loaded: bool,
    // Pagination
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for SalesRegisterState {
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
            marketplace: String::new(),
            sort_field: "sale_date".to_string(),
            sort_ascending: false,
            is_loaded: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<SalesRegisterState> {
    RwSignal::new(SalesRegisterState::default())
}
