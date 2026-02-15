use super::WbOrdersDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct WbOrdersState {
    pub orders: Vec<WbOrdersDto>,
    pub date_from: String,
    pub date_to: String,
    pub selected_organization_id: Option<String>,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: HashSet<String>,
    pub is_loaded: bool,
    // Search fields
    pub search_query: String,
    // Filter fields
    pub show_cancelled: bool,
    // Pagination fields
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for WbOrdersState {
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
            orders: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            selected_organization_id: None,
            sort_field: "order_date".to_string(),
            sort_ascending: false, // Newest first
            selected_ids: HashSet::new(),
            is_loaded: false,
            search_query: String::new(),
            // Filter defaults
            show_cancelled: true, // Show all by default
            // Pagination defaults
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

/// Create state signal
pub fn create_state() -> RwSignal<WbOrdersState> {
    RwSignal::new(WbOrdersState::default())
}
