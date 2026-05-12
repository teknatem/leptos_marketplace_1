use super::WbReturnsClaimsListDto;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct WbReturnsClaimsState {
    pub items: Vec<WbReturnsClaimsListDto>,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub search_query: String,
    pub is_loaded: bool,
    pub show_archived: bool,
    /// Пустой вектор = все статусы; иначе — только выбранные
    pub selected_statuses: Vec<i32>,
}

impl Default for WbReturnsClaimsState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            sort_field: "dt".to_string(),
            sort_ascending: false,
            search_query: String::new(),
            is_loaded: false,
            show_archived: true,
            selected_statuses: Vec::new(),
        }
    }
}

pub fn create_state() -> RwSignal<WbReturnsClaimsState> {
    RwSignal::new(WbReturnsClaimsState::default())
}
