use leptos::prelude::*;
use std::collections::HashSet;

use super::MarketplaceSalesRow;

#[derive(Clone, Debug)]
pub struct MarketplaceSalesState {
    pub items: Vec<MarketplaceSalesRow>,
    pub search_query: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: HashSet<String>,
    pub is_loaded: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for MarketplaceSalesState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            search_query: String::new(),
            sort_field: "accrual_date".to_string(),
            sort_ascending: false,
            selected_ids: HashSet::new(),
            is_loaded: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<MarketplaceSalesState> {
    RwSignal::new(MarketplaceSalesState::default())
}
