use super::OzonTransactionsDto;
use chrono::{Datelike, Utc};
use leptos::prelude::*;

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
    pub selected_ids: Vec<String>,
    pub is_loaded: bool,
}

impl Default for TransactionsState {
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
            transactions: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            transaction_type_filter: "".to_string(),
            operation_type_name_filter: "".to_string(),
            posting_number_filter: "".to_string(),
            sort_field: "operation_date".to_string(),
            sort_ascending: false,
            selected_ids: Vec::new(),
            is_loaded: false,
        }
    }
}

// Create state within component scope instead of thread-local
// This ensures state is properly disposed when component unmounts
pub fn create_state() -> RwSignal<TransactionsState> {
    RwSignal::new(TransactionsState::default())
}
