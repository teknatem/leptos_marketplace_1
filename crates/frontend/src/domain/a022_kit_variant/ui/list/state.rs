use super::KitVariantDto;
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct KitVariantState {
    pub items: Vec<KitVariantDto>,
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

impl Default for KitVariantState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            search_query: String::new(),
            sort_field: "description".to_string(),
            sort_ascending: true,
            selected_ids: HashSet::new(),
            is_loaded: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<KitVariantState> {
    RwSignal::new(KitVariantState::default())
}
