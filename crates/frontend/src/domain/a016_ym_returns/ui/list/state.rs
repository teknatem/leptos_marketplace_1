use chrono::{Datelike, Utc};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct YmReturnsState {
    // Фильтры
    pub date_from: String,
    pub date_to: String,
    pub filter_type: Option<String>,

    // Сортировка
    pub sort_field: String,
    pub sort_ascending: bool,

    // Множественный выбор
    pub selected_ids: HashSet<String>,

    // Флаг загрузки
    pub is_loaded: bool,

    // Серверная пагинация
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,

    // Поиск
    pub search_return_id: String,
    pub search_order_id: String,

    // Серверные итоги
    pub server_totals: Option<ServerTotals>,
}

/// Серверные итоги по датасету
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerTotals {
    pub total_records: usize,
    pub sum_items: i32,
    pub sum_amount: f64,
    pub returns_count: usize,
    pub unredeemed_count: usize,
}

impl Default for YmReturnsState {
    fn default() -> Self {
        // Период по умолчанию: текущий месяц
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
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            filter_type: None,
            sort_field: "created_at_source".to_string(),
            sort_ascending: false,
            selected_ids: HashSet::new(),
            is_loaded: false,
            // Пагинация
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
            // Поиск
            search_return_id: String::new(),
            search_order_id: String::new(),
            // Итоги
            server_totals: None,
        }
    }
}

pub fn create_state() -> RwSignal<YmReturnsState> {
    RwSignal::new(YmReturnsState::default())
}
