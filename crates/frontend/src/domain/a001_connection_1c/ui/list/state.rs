use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct Connection1CState {
    pub items: Vec<Connection1CDatabase>,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: Vec<String>,
    pub is_loaded: bool,
    // Серверная пагинация
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for Connection1CState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            sort_field: "description".to_string(),
            sort_ascending: true, // A-Z по умолчанию
            selected_ids: Vec::new(),
            is_loaded: false,
            // Пагинация
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<Connection1CState> {
    RwSignal::new(Connection1CState::default())
}

