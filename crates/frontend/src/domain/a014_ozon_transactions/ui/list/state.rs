use super::OzonTransactionsDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct TransactionsState {
    pub transactions: Vec<OzonTransactionsDto>,
    pub date_from: String,
    pub date_to: String,
    pub transaction_type_filter: String,
    pub operation_type_name_filter: String,
    pub posting_number_filter: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: HashSet<String>,
    pub is_loaded: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for TransactionsState {
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
            transactions: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            transaction_type_filter: String::new(),
            operation_type_name_filter: String::new(),
            posting_number_filter: String::new(),
            sort_field: "operation_date".to_string(),
            sort_ascending: false,
            selected_ids: HashSet::new(),
            is_loaded: false,
            page: 0,
            page_size: 200,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<TransactionsState> {
    RwSignal::new(TransactionsState::default())
}
