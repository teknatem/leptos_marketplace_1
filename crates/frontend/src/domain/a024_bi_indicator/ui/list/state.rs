use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct BiIndicatorListState {
    pub q: String,

    pub sort_field: String,
    pub sort_ascending: bool,

    pub selected_ids: Vec<String>,

    pub is_loaded: bool,

    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for BiIndicatorListState {
    fn default() -> Self {
        Self {
            q: String::new(),
            sort_field: "code".to_string(),
            sort_ascending: true,
            selected_ids: Vec::new(),
            is_loaded: false,
            page: 0,
            page_size: 50,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<BiIndicatorListState> {
    RwSignal::new(BiIndicatorListState::default())
}
