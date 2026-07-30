use contracts::system::tickets::TicketDto;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct TicketsListState {
    pub items: Vec<TicketDto>,
    pub search_query: String,
    pub status_filter: String,
    pub type_filter: String,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub is_loaded: bool,
}

impl Default for TicketsListState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            search_query: String::new(),
            status_filter: String::new(),
            type_filter: String::new(),
            page: 0,
            page_size: 50,
            total_count: 0,
            total_pages: 1,
            is_loaded: false,
        }
    }
}

pub fn create_state() -> RwSignal<TicketsListState> {
    RwSignal::new(TicketsListState::default())
}
