use super::WbDayCloseListDto;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct WbDayCloseListState {
    pub items: Vec<WbDayCloseListDto>,
    pub is_loaded: bool,
    pub show_archived: bool,
    pub filter_connection_id: String,
    pub filter_date_from: String,
    pub filter_date_to: String,
    pub sort_field: String,
    pub sort_ascending: bool,
}

impl Default for WbDayCloseListState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            is_loaded: false,
            show_archived: false,
            filter_connection_id: String::new(),
            filter_date_from: String::new(),
            filter_date_to: String::new(),
            sort_field: "business_date".to_string(),
            sort_ascending: false,
        }
    }
}

pub fn create_state() -> RwSignal<WbDayCloseListState> {
    RwSignal::new(WbDayCloseListState::default())
}
