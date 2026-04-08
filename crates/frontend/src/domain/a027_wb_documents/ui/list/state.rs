use super::WbDocumentsListDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct WbDocumentsListState {
    pub items: Vec<WbDocumentsListDto>,
    pub date_from: String,
    pub date_to: String,
    pub selected_connection_id: Option<String>,
    pub weekly_only: bool,
    pub search_query: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub is_loaded: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for WbDocumentsListState {
    fn default() -> Self {
        let now = Utc::now().date_naive();
        let year = now.year();
        let month = now.month();
        let month_start =
            chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("invalid month start");
        let month_end = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("invalid month end")
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("invalid month end")
        };

        Self {
            items: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            selected_connection_id: None,
            weekly_only: false,
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

pub fn create_state() -> RwSignal<WbDocumentsListState> {
    RwSignal::new(WbDocumentsListState::default())
}
