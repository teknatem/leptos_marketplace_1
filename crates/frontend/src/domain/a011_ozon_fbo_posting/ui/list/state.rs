use leptos::prelude::*;
use std::collections::HashSet;

use super::OzonFboPostingDto;

#[derive(Clone, Debug)]
pub struct OzonFboPostingState {
    pub items: Vec<OzonFboPostingDto>,
    pub status_filter: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: HashSet<String>,
    pub is_loaded: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for OzonFboPostingState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            status_filter: String::new(),
            sort_field: "document_no".to_string(),
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

pub fn create_state() -> RwSignal<OzonFboPostingState> {
    RwSignal::new(OzonFboPostingState::default())
}
