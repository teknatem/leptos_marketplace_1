use super::MissingCostRegistryListItemDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct MissingCostRegistryListState {
    pub items: Vec<MissingCostRegistryListItemDto>,
    pub date_from: String,
    pub date_to: String,
    pub search_query: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub is_loaded: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for MissingCostRegistryListState {
    fn default() -> Self {
        let now = Utc::now().date_naive();
        let year = now.year();
        let year_start = chrono::NaiveDate::from_ymd_opt(year, 1, 1).expect("Invalid year start");
        let year_end = chrono::NaiveDate::from_ymd_opt(year, 12, 31).expect("Invalid year end");

        Self {
            items: Vec::new(),
            date_from: year_start.format("%Y-%m-%d").to_string(),
            date_to: year_end.format("%Y-%m-%d").to_string(),
            search_query: String::new(),
            sort_field: "document_date".to_string(),
            sort_ascending: false,
            is_loaded: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<MissingCostRegistryListState> {
    RwSignal::new(MissingCostRegistryListState::default())
}
