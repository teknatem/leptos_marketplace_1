use contracts::domain::a013_ym_order::aggregate::YmOrderListDto;
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct YmOrderState {
    pub orders: Vec<YmOrderListDto>,
    pub date_from: String,
    pub date_to: String,
    pub selected_organization_id: Option<String>,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: HashSet<String>,
    pub is_loaded: bool,
    // Pagination fields
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    // Search fields
    pub search_order_no: String,
    pub filter_status: String,
}

impl Default for YmOrderState {
    fn default() -> Self {
        Self {
            orders: Vec::new(),
            date_from: String::new(), // Пустая дата - фильтр не применяется
            date_to: String::new(),   // Пустая дата - фильтр не применяется
            selected_organization_id: None,
            sort_field: "delivery_date".to_string(),
            sort_ascending: false,
            selected_ids: HashSet::new(),
            is_loaded: false,
            // Pagination defaults
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
            // Search defaults
            search_order_no: String::new(),
            filter_status: String::new(),
        }
    }
}

// Create state within component scope instead of thread-local
// This ensures state is properly disposed when component unmounts
pub fn create_state() -> RwSignal<YmOrderState> {
    RwSignal::new(YmOrderState::default())
}
