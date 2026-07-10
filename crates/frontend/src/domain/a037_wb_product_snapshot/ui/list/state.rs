use super::WbProductSnapshotListDto;
use chrono::Utc;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct WbProductSnapshotListState {
    pub items: Vec<WbProductSnapshotListDto>,
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

impl Default for WbProductSnapshotListState {
    fn default() -> Self {
        // Дефолт — последние 30 дней снимков (динамика набирается со временем).
        let today = Utc::now().date_naive();
        let month_ago = today - chrono::Duration::days(29);

        Self {
            items: Vec::new(),
            date_from: month_ago.format("%Y-%m-%d").to_string(),
            date_to: today.format("%Y-%m-%d").to_string(),
            selected_connection_id: None,
            search_query: String::new(),
            sort_field: "document_date".to_string(),
            sort_ascending: false,
            is_loaded: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<WbProductSnapshotListState> {
    RwSignal::new(WbProductSnapshotListState::default())
}
