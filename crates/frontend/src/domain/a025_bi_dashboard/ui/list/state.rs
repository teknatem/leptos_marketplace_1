use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct BiDashboardListState {
    pub query: RwSignal<String>,
    pub sort_by: RwSignal<String>,
    pub sort_desc: RwSignal<bool>,
    pub selected_ids: RwSignal<Vec<String>>,
    pub page: RwSignal<usize>,
    pub page_size: RwSignal<usize>,
    pub total_count: RwSignal<u64>,
    pub total_pages: RwSignal<usize>,
    pub is_loaded: RwSignal<bool>,
}

pub fn create_state() -> BiDashboardListState {
    BiDashboardListState {
        query: RwSignal::new(String::new()),
        sort_by: RwSignal::new("created_at".to_string()),
        sort_desc: RwSignal::new(true),
        selected_ids: RwSignal::new(vec![]),
        page: RwSignal::new(0),
        page_size: RwSignal::new(50),
        total_count: RwSignal::new(0),
        total_pages: RwSignal::new(0),
        is_loaded: RwSignal::new(false),
    }
}
