use super::WbPromotionDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct WbPromotionState {
    pub promotions: Vec<WbPromotionDto>,
    pub date_from: String,
    pub date_to: String,
    pub selected_connection_id: Option<String>,
    pub search_query: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub is_loaded: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for WbPromotionState {
    fn default() -> Self {
        let now = Utc::now().date_naive();
        let year = now.year();
        let month = now.month();
        let month_start =
            chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid month start date");
        // Show 3 months ahead for promotions
        let month_end = if month >= 10 {
            chrono::NaiveDate::from_ymd_opt(year + 1, (month + 3) % 12, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("Invalid month end date")
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 3, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("Invalid month end date")
        };

        Self {
            promotions: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            selected_connection_id: None,
            search_query: String::new(),
            sort_field: "start_date_time".to_string(),
            sort_ascending: true,
            is_loaded: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<WbPromotionState> {
    RwSignal::new(WbPromotionState::default())
}
