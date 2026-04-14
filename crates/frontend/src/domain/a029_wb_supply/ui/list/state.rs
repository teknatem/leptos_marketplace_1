use super::WbSupplyDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct WbSupplyState {
    pub supplies: Vec<WbSupplyDto>,
    pub date_from: String,
    pub date_to: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: HashSet<String>,
    pub selected_organization_id: Option<String>,
    pub is_loaded: bool,
    pub search_query: String,
    pub show_done: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for WbSupplyState {
    fn default() -> Self {
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
            supplies: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            sort_field: "created_at_wb".to_string(),
            sort_ascending: false,
            selected_ids: HashSet::new(),
            selected_organization_id: None,
            is_loaded: false,
            search_query: String::new(),
            show_done: true,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<WbSupplyState> {
    RwSignal::new(WbSupplyState::default())
}
